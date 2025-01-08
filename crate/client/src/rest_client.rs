use std::fmt::Display;

use crate::{
    InstantiatedFindex,
    config::FindexClientConfig,
    error::{
        FindexClientError,
        result::{FindexClientResult, FindexRestClientResultHelper},
    },
};
use cosmian_findex::{ADDRESS_LENGTH, Address, Findex, KEY_LENGTH, MemoryADT, Secret};
use cosmian_findex_structs::{
    Addresses, Guard, OptionalWords, Tasks, WORD_LENGTH, decode_fn, encode_fn,
};
use cosmian_http_client::HttpClient;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace, warn};
use uuid::Uuid;

// Response for success
#[derive(Deserialize, Serialize, Debug)] // Debug is required by ok_json()
pub struct SuccessResponse {
    pub success: String,
    pub index_id: Uuid,
}

impl Display for SuccessResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.success)
    }
}

#[derive(Clone)]
pub struct FindexRestClient {
    pub client: HttpClient,
    pub config: FindexClientConfig,
    index_id: Option<String>,
}

impl FindexRestClient {
    /// Initialize a Findex REST client.
    ///
    /// Parameters `server_url` and `accept_invalid_certs` from the command line
    /// will override the ones from the configuration file.
    /// # Errors
    /// Return an error if the configuration file is not found or if the
    /// configuration is invalid or if the client cannot be instantiated.
    pub fn new(config: FindexClientConfig) -> Result<Self, FindexClientError> {
        // Instantiate a Findex server REST client with the given configuration
        let client = HttpClient::instantiate(&config.http_config).with_context(|| {
            format!(
                "Unable to instantiate a Findex REST client to server at {}",
                config.http_config.server_url
            )
        })?;

        Ok(Self {
            client,
            config,
            index_id: None,
        })
    }
    /// Instantiate a Findex REST client with a specific index. See below. Do not expose this.
    fn new_memory(&self, index_id: Uuid) -> FindexRestClient {
        Self {
            client: self.client.clone(), // TODO(review): is cloning ok  here ?
            config: self.config.clone(),
            index_id: Some(index_id.to_string()),
        }
    }
    /// Instantiate a Findex REST client with a specific index.
    /// Batch read and guarded write operations are defined in the findex crate and thus their signatures are not editable
    /// Ideal way to access the Some(index_id) is by having it as a field in the FindexRestClient struct
    /// In the cli crate, first instantiate a base FindexRestClient and that will be used to instantiate a FindexRestClient with a specific index
    /// each time a call for Findex is needed
    pub fn instantiate_findex(
        &self,
        index_id: &Uuid,
        key: &Secret<KEY_LENGTH>,
    ) -> Result<InstantiatedFindex, FindexClientError> {
        trace!("Instantiating a Findex rest client");
        Ok(Findex::new(
            key, // TODO(review): is cloning ok here ?
            self.new_memory(*index_id),
            encode_fn::<WORD_LENGTH, _>,
            decode_fn,
        ))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn version(&self) -> FindexClientResult<String> {
        let endpoint = "/version";
        let server_url = format!("{}{endpoint}", self.client.server_url);
        let response = self.client.client.get(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<String>().await?);
        }

        // process error
        let p = handle_error(endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }
}

impl MemoryADT for FindexRestClient {
    type Address = Address<ADDRESS_LENGTH>;
    type Word = [u8; WORD_LENGTH];
    type Error = FindexClientError;

    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<[u8; WORD_LENGTH]>>, FindexClientError> {
        let index_id = self.index_id.clone().expect(
            "Unexpected error : this function should never be called while from base instance",
        );
        let endpoint = format!("/indexes/{}/batch_read", index_id);
        let server_url = format!("{}{}", self.client.server_url, endpoint);
        trace!(
            "Initiating batch_read of {} addresses for index {} at server_url: {}",
            addresses.len(),
            index_id,
            server_url
        );
        // Convert addresses to bytes
        let request_bytes = Addresses::new(addresses).serialize()?;

        let response = self
            .client
            .client
            .post(server_url.clone())
            .body(request_bytes)
            .send()
            .await?;
        let status_code = response.status();
        if status_code.is_success() {
            // request successful, decode the response using same encoding protocol
            let bytes = response.bytes().await?.to_vec();
            let result = OptionalWords::<WORD_LENGTH>::deserialize(bytes)?;
            trace!(
                "batch_read successful on server url {:?}. result: {:?}",
                server_url.clone(),
                result
            );
            return Ok(result.into_inner());
        }

        // process error
        warn!("batch_read failed on server url {:?}.", server_url);
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        tasks: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<[u8; WORD_LENGTH]>, FindexClientError> {
        let index_id = self.index_id.clone().expect(
            "Unexpected error : this function should never be called while from base instance",
        );
        let endpoint = format!("/indexes/{}/guarded_write", index_id);
        let server_url = format!("{}{}", self.client.server_url, endpoint.clone());
        trace!(
            "Initiating guarded_write of {} values for index {} at server_url: {}",
            tasks.len(),
            index_id,
            server_url.clone()
        );

        // code the request body
        let guard_bytes = Guard::new(guard.0, guard.1).serialize()?;
        let task_bytes = Tasks::new(tasks).serialize()?;

        // Merge the two vectors into one
        let mut request_bytes = Vec::with_capacity(guard_bytes.len() + task_bytes.len());
        request_bytes.extend_from_slice(&guard_bytes);
        request_bytes.extend_from_slice(&task_bytes);

        let response = self
            .client
            .client
            .post(server_url.clone())
            .body(request_bytes)
            .send()
            .await?;

        let status_code = response.status();

        if status_code.is_success() {
            // request successful, decode the response using same encoding protocol defined in crate/server/src/routes/findex.rs
            let bytes = response.bytes().await?;
            let result_word =
                OptionalWords::<WORD_LENGTH>::deserialize(bytes.to_vec())?.into_inner();
            if result_word.len() != 1 {
                return Err(FindexClientError::RequestFailed(format!(
                    "Unexpected response from server. Expected 1 word, got {}",
                    result_word.len()
                )));
            } else {
                trace!(
                    "guarded_write successful on server url {:?}. result_word: {:?}",
                    server_url.clone(),
                    result_word
                );
                return Ok(result_word[0]);
            }
        }
        // process error
        warn!("guarded_write failed on server url {}.", server_url);
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }
}

/// Handle the status code of the response.
pub(crate) async fn handle_status_code(
    response: Response,
    endpoint: &str,
) -> FindexClientResult<SuccessResponse> {
    if response.status().is_success() {
        Ok(response.json::<SuccessResponse>().await?)
    } else {
        let p = handle_error(endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }
}

/// Some errors are returned by the Middleware without going through our own
/// error manager. In that case, we make the error clearer here for the client.
/// # Errors
/// Return an error if the response cannot be read.
/// Return an error if the response is not a success.
pub async fn handle_error(endpoint: &str, response: Response) -> Result<String, FindexClientError> {
    trace!("Error response received on {endpoint}: Response: {response:?}");
    let status = response.status();
    let text = response.text().await?;

    Ok(format!(
        "{}: {}",
        endpoint,
        if text.is_empty() {
            match status {
                StatusCode::NOT_FOUND => "Findex server endpoint does not exist".to_owned(),
                StatusCode::UNAUTHORIZED => "Bad authorization token".to_owned(),
                _ => format!("{status} {text}"),
            }
        } else {
            text
        }
    ))
}
