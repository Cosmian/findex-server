use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
    time::Duration,
};

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Identity, Response, StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};

use crate::{
    error::{result::ClientResult, ClientError},
    ClientResultHelper,
};

#[derive(Clone)]
pub struct RestClient {
    pub server_url: String,
    pub client: Client,
}

impl RestClient {
    /// Instantiate a new Findex REST Client
    /// # Errors
    /// It returns an error if the client cannot be instantiated
    pub fn instantiate(
        server_url: &str,
        bearer_token: Option<&str>,
        ssl_client_pkcs12_path: Option<&str>,
        ssl_client_pkcs12_password: Option<&str>,
        accept_invalid_certs: bool,
    ) -> Result<Self, ClientError> {
        let server_url = server_url
            .strip_suffix('/')
            .map_or_else(|| server_url.to_owned(), std::string::ToString::to_string);

        let mut headers = HeaderMap::new();
        if let Some(bearer_token) = bearer_token {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(format!("Bearer {bearer_token}").as_str())?,
            );
        }

        // We deal with 4 scenarios:
        // 1. HTTP: no TLS
        // 2. HTTPS: a) self-signed: we want to remove the verifications b) signed in a
        //    non-tee context: we want classic TLS verification based on the root ca
        let builder = ClientBuilder::new().danger_accept_invalid_certs(accept_invalid_certs);
        // If a PKCS12 file is provided, use it to build the client
        let builder = match ssl_client_pkcs12_path {
            Some(ssl_client_pkcs12) => {
                let mut pkcs12 = BufReader::new(File::open(ssl_client_pkcs12)?);
                let mut pkcs12_bytes = vec![];
                pkcs12.read_to_end(&mut pkcs12_bytes)?;
                let pkcs12 = Identity::from_pkcs12_der(
                    &pkcs12_bytes,
                    ssl_client_pkcs12_password.unwrap_or(""),
                )?;
                builder.identity(pkcs12)
            }
            None => builder,
        };

        // Build the client
        Ok(Self {
            client: builder
                .default_headers(headers)
                .tcp_keepalive(Duration::from_secs(60))
                .build()
                .context("Reqwest client builder")?,
            server_url,
        })
    }

    /// This operation requests the server to create a new table.
    /// The returned secrets could be shared between several users.
    /// # Errors
    /// It returns an error if the request fails
    #[instrument(ret(Display), err, skip(self))]
    pub async fn version(&self) -> ClientResult<String> {
        let endpoint = "/version";
        let server_url = format!("{}{endpoint}", self.server_url);
        let response = self.client.get(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<String>().await?);
        }

        // process error
        let p = handle_error(endpoint, response).await?;
        Err(ClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn create_access(&self) -> ClientResult<SuccessResponse> {
        let endpoint = "/access/create".to_owned();
        let server_url = format!("{}{endpoint}", self.server_url);
        trace!("POST create_access: {server_url}");
        let response = self.client.post(server_url).send().await?;
        trace!("Response: {response:?}");
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(ClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn grant_access(
        &self,
        user_id: &str,
        permission: &str,
        index_id: &str,
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/access/grant/{user_id}/{permission}/{index_id}");
        let server_url = format!("{}{endpoint}", self.server_url);
        trace!("POST grant_access: {server_url}");
        let response = self.client.post(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(ClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn revoke_access(
        &self,
        user_id: &str,
        index_id: &str,
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/access/revoke/{user_id}/{index_id}");
        let server_url = format!("{}{endpoint}", self.server_url);
        trace!("POST revoke_access: {server_url}");
        let response = self.client.post(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(ClientError::RequestFailed(p))
    }
}

/// Some errors are returned by the Middleware without going through our own
/// error manager. In that case, we make the error clearer here for the client.
async fn handle_error(endpoint: &str, response: Response) -> Result<String, ClientError> {
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

#[derive(Deserialize, Serialize, Debug)] // Debug is required by ok_json()
pub struct SuccessResponse {
    pub success: String,
}

impl Display for SuccessResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.success)
    }
}
