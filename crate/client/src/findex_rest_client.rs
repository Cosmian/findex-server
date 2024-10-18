use std::{
    fs::File,
    io::{BufReader, Read},
    sync::Arc,
    time::Duration,
};

use log::trace;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Identity, Response, StatusCode,
};
use rustls::{client::WebPkiVerifier, Certificate};

use crate::{
    certificate_verifier::{LeafCertificateVerifier, NoVerifier},
    error::{result::ClientResult, ClientError},
    ClientResultHelper,
};

#[derive(Clone)]
pub struct FindexClient {
    pub server_url: String,
    pub client: Client,
}

impl FindexClient {
    /// Instantiate a new Findex REST Client
    /// # Errors
    /// It returns an error if the client cannot be instantiated
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub fn instantiate(
        server_url: &str,
        bearer_token: Option<&str>,
        ssl_client_pkcs12_path: Option<&str>,
        ssl_client_pkcs12_password: Option<&str>,
        database_secret: Option<&str>,
        accept_invalid_certs: bool,
        allowed_tee_tls_cert: Option<Certificate>,
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
        if let Some(database_secret) = database_secret {
            headers.insert(
                "FindexDatabaseSecret",
                HeaderValue::from_str(database_secret)?,
            );
        }

        // We deal with 4 scenarios:
        // 1. HTTP: no TLS
        // 2. HTTPS: a) self-signed: we want to remove the verifications b) signed in a
        //    tee context: we want to verify the /quote and then only accept the allowed
        //    certificate -> For efficiency purpose, this verification is made outside
        //    this call (async with the queries) Only the verified certificate is used
        //    here c) signed in a non-tee context: we want classic TLS verification
        //    based on the root ca
        let builder = allowed_tee_tls_cert.map_or_else(
            || ClientBuilder::new().danger_accept_invalid_certs(accept_invalid_certs),
            |certificate| build_tls_client_tee(certificate, accept_invalid_certs),
        );

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

    /// This operation requests the server to create a new database.
    /// The returned secrets could be shared between several users.
    /// # Errors
    /// It returns an error if the request fails
    pub async fn new_database(&self) -> ClientResult<String> {
        let endpoint = "/new_database";
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

    /// This operation requests the server to create a new table.
    /// The returned secrets could be shared between several users.
    /// # Errors
    /// It returns an error if the request fails
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

/// Build a `TLSClient` to use with a Findex running inside a tee.
/// The TLS verification is the basic one but also includes the verification of
/// the leaf certificate The TLS socket is mounted since the leaf certificate is
/// exactly the same as the expected one.
pub(crate) fn build_tls_client_tee(
    leaf_cert: Certificate,
    accept_invalid_certs: bool,
) -> ClientBuilder {
    let mut root_cert_store = rustls::RootCertStore::empty();

    let trust_anchors = webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|trust_anchor| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            trust_anchor.subject,
            trust_anchor.spki,
            trust_anchor.name_constraints,
        )
    });
    root_cert_store.add_trust_anchors(trust_anchors);

    let verifier = if accept_invalid_certs {
        LeafCertificateVerifier::new(leaf_cert, Arc::new(NoVerifier))
    } else {
        LeafCertificateVerifier::new(
            leaf_cert,
            Arc::new(WebPkiVerifier::new(root_cert_store, None)),
        )
    };

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();

    // Create a client builder
    Client::builder().use_preconfigured_tls(config)
}
