use std::fmt::Display;

use cosmian_findex::{mem::MemoryError, Address, MemoryADT, ADDRESS_LENGTH};
use cosmian_findex_config::FindexClientConfig;
use cosmian_findex_server::database::redis::WORD_LENGTH;
use cosmian_http_client::HttpClient;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};

use crate::error::{
    result::{FindexClientResult, FindexRestClientResultHelper},
    FindexClientError,
};

// Response for success
#[derive(Deserialize, Serialize, Debug)] // Debug is required by ok_json()
pub struct SuccessResponse {
    pub success: String,
}

impl Display for SuccessResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.success)
    }
}

#[derive(Clone)]
pub struct FindexRestClient {
    pub client: HttpClient,
    pub conf: FindexClientConfig,
}

impl FindexRestClient {
    /// Initialize a Findex REST client.
    ///
    /// Parameters `server_url` and `accept_invalid_certs` from the command line
    /// will override the ones from the configuration file.
    pub fn new(conf: FindexClientConfig) -> Result<FindexRestClient, FindexClientError> {
        // Instantiate a Findex server REST client with the given configuration
        let client = HttpClient::instantiate(&conf.http_config).with_context(|| {
            format!(
                "Unable to instantiate a Findex REST client to server at {}",
                conf.http_config.server_url
            )
        })?;

        Ok(Self { client, conf })
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
    type Word = [u8; WORD_LENGTH]; // TODO(hatem): un-hard code this
    type Error = FindexClientError;

    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<[u8; WORD_LENGTH]>>, FindexClientError>
/* -> impl Send + std::future::Future<Output = Result<Vec<Option<Self::Word>>, Self::Error>> */
    {
        let endpoint = "/indexes/{index_id}/batch_read";
        let server_url = format!("{}{endpoint}", self.client.server_url);
        // Convert addresses to bytes
        let request_bytes = addresses
            .into_iter()
            .flat_map(|addr| <[u8; ADDRESS_LENGTH]>::from(addr)) // TODO: is flat map ok ?
            .collect::<Vec<u8>>();

        let response = self
            .client
            .client
            .post(server_url)
            .body(request_bytes)
            .send()
            .await?;
        let status_code = response.status();
        if status_code.is_success() {
            // request successful, decode the response using same encoding protocol defined in crate/server/src/routes/findex.rs
            let bytes = response.bytes().await?;
            let mut result = Vec::new();
            let mut pos = 0;

            while pos < bytes.len() {
                if bytes[pos] == 0 {
                    result.push(None);
                    pos += 1;
                } else {
                    let word_bytes = &bytes[pos + 1..pos + 1 + WORD_LENGTH];
                    let mut word = [0u8; WORD_LENGTH];
                    word.copy_from_slice(word_bytes);
                    result.push(Some(word));
                    pos += 130; // 1 (discriminant) + WORD_LENGTH (word)
                }
            }

            return Ok(result);
        }

        // process error
        let p = handle_error(endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        tasks: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<[u8; WORD_LENGTH]>, FindexClientError> {
        let endpoint = "/indexes/{index_id}/batch_read";
        let server_url = format!("{}{endpoint}", self.client.server_url);

        // code the request body
        let mut request_bytes = Vec::new();
        request_bytes.extend_from_slice(&<[u8; ADDRESS_LENGTH]>::from(guard.0)); // Add guard address
        match guard.1 {
            // Add guard word with option flag
            Some(word) => {
                request_bytes.push(1); // Some
                request_bytes.extend_from_slice(&word);
            }
            None => {
                request_bytes.push(0); // None
            }
        }
        for (addr, word) in tasks {
            // Add tasks
            request_bytes.extend_from_slice(&<[u8; ADDRESS_LENGTH]>::from(addr));
            request_bytes.extend_from_slice(&word);
        }

        let response = self
            .client
            .client
            .post(server_url)
            .body(request_bytes)
            .send()
            .await?;

        let status_code = response.status();

        if status_code.is_success() {
            // request successful, decode the response using same encoding protocol defined in crate/server/src/routes/findex.rs
            let bytes = response.bytes().await?;
            let result_word = if bytes[0] == 0 {
                None
            } else {
                let word_bytes = &bytes[1..1 + WORD_LENGTH];
                let mut word = [0u8; WORD_LENGTH];
                word.copy_from_slice(word_bytes);
                Some(word)
            };
            return Ok(result_word);
        }
        // process error
        let p = handle_error(endpoint, response).await?;
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
