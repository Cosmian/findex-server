use crate::{
    config::FindexClientConfig,
    error::{
        result::{FindexClientResult, FindexRestClientResultHelper},
        FindexClientError,
    },
    InstantiatedFindex, WORD_LENGTH,
};
use base64::{engine::general_purpose, Engine};
use cosmian_findex::{
    generic_decode, generic_encode, Address, Findex, MemoryADT, Secret, ADDRESS_LENGTH, KEY_LENGTH,
};
use cosmian_findex_structs::{Addresses, Guard, OptionalWords, Tasks};
use cosmian_http_client::HttpClient;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tracing::{trace, warn};
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
    pub http_client: HttpClient,
    /// Each instance of FindexRestClient is associated with a specific index
    /// however, since all RestClient are instantiated with the same http_client,
    /// we keep the client in the base struct and create each new index_id bounded
    /// InstantiatedFindex after providing its index_id via the instantiate_findex
    /// function.
    index_id: Option<Uuid>,
}

impl FindexRestClient {
    /// Initialize a Findex REST client.
    ///
    /// Parameters `server_url` and `accept_invalid_certs` from the command line
    /// will override the ones from the configuration file.
    /// # Errors
    /// Return an error if the configuration file is not found or if the
    /// configuration is invalid or if the client cannot be instantiated.
    pub fn new(config: &FindexClientConfig) -> Result<Self, FindexClientError> {
        // Instantiate a Findex server REST client with the given configuration
        let client = HttpClient::instantiate(&config.http_config).with_context(|| {
            format!(
                "Unable to instantiate a Findex REST client to server at {}",
                config.http_config.server_url
            )
        })?;

        Ok(Self {
            http_client: client,
            index_id: None,
        })
    }

    /// Instantiate a Findex REST client with a specific index.
    fn new_with_index_id(self, index_id: Uuid) -> Self {
        Self {
            http_client: self.http_client,
            index_id: Some(index_id),
        }
    }

    /// Instantiate a Findex REST client with a specific index. In the CLI
    /// crate, first instantiate a base `FindexRestClient` and that will be used
    /// to instantiate a findex instance with a specific index each time a call
    /// for Findex is needed
    ///
    /// # Errors
    /// Return an error if the Findex cannot be instantiated.
    pub fn instantiate_findex(
        self,
        index_id: Uuid,
        seed: &Secret<KEY_LENGTH>,
    ) -> Result<InstantiatedFindex, FindexClientError> {
        trace!("Instantiating a Findex rest client");
        Ok(Findex::new(
            seed,
            self.new_with_index_id(index_id),
            generic_encode,
            generic_decode,
        ))
    }

    // #[instrument(ret(Display), err, skip(self))]
    /// # Errors
    /// Return an error if the request fails.
    pub async fn version(&self) -> FindexClientResult<String> {
        let endpoint = "/version";
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        let response = self.http_client.client.get(server_url).send().await?;
        if response.status().is_success() {
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

    #[allow(clippy::renamed_function_params)] // original name (a) is less clear
    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<[u8; WORD_LENGTH]>>, FindexClientError> {
        let index_id = self
            .index_id
            .ok_or_else(|| FindexClientError::Default("index ID is missing".to_owned()))?;

        let endpoint = format!("/indexes/{index_id}/batch_read");
        let server_url = format!("{}{}", self.http_client.server_url, endpoint);

        trace!(
            "Initiating batch_read of {} addresses for index {} at server_url: {}",
            addresses.len(),
            index_id,
            server_url
        );

        let response = self
            .http_client
            .client
            .post(&server_url)
            .body(Addresses::new(addresses).serialize()?)
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("batch_read failed on server url {:?}.", server_url);
            let err = handle_error(&endpoint, response).await?;
            return Err(FindexClientError::RequestFailed(err));
        }

        let words: OptionalWords<WORD_LENGTH> =
            OptionalWords::deserialize(&response.bytes().await?)?;

        trace!(
            "batch_read successful on server url {}. result: {}",
            &server_url,
            words
        );

        Ok(words.into_inner())
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        tasks: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<[u8; WORD_LENGTH]>, FindexClientError> {
        let index_id = self
            .index_id
            .ok_or_else(|| FindexClientError::Default("index ID is missing".to_owned()))?;

        let endpoint = format!("/indexes/{index_id}/guarded_write");
        let server_url = format!("{}{}", self.http_client.server_url, &endpoint);

        trace!(
            "Initiating guarded_write of {} values for index {} at server_url: {}",
            tasks.len(),
            index_id,
            &server_url
        );

        // BEGIN TODO: using `Serializable` avoids re-coding vector
        // concatenation. Anyway, this should be abstracted away in a function.
        let guard_bytes = Guard::new(guard.0, guard.1).serialize()?;
        let task_bytes = Tasks::new(tasks).serialize()?;
        let mut request_bytes = Vec::with_capacity(guard_bytes.len() + task_bytes.len());
        request_bytes.extend_from_slice(&guard_bytes);
        request_bytes.extend_from_slice(&task_bytes);
        // END TODO

        let response = self
            .http_client
            .client
            .post(&server_url)
            .body(request_bytes)
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("guarded_write failed on server url {}.", server_url);
            let err = handle_error(&endpoint, response).await?;
            return Err(FindexClientError::RequestFailed(err));
        }

        let guard = {
            let bytes = response.bytes().await?;
            let words: Vec<_> = OptionalWords::deserialize(&bytes)?.into();
            words.first().copied().ok_or_else(|| {
                FindexClientError::RequestFailed(format!(
                    "Unexpected response from server. Expected 1 word, got {}",
                    words.len()
                ))
            })
        }?;

        trace!(
            "guarded_write successful on server url {}. guard: {}",
            server_url,
            guard.map_or("None".to_owned(), |g| general_purpose::STANDARD.encode(g))
        );

        Ok(guard)
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
