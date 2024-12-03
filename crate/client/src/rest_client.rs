use std::fmt::Display;

use cosmian_findex_config::FindexClientConfig;
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
