use std::{fmt, path::PathBuf};

use openssl::x509::X509;
use tracing::warn;

use super::{DbParams, HttpParams};
use crate::{
    config::{ClapConfig, IdpConfig},
    error::result::FResult,
    server_bail,
};

/// This structure is the context used by the server
/// while it is running. There is a singleton instance
/// shared between all threads.
pub struct ServerParams {
    /// The JWT Config if Auth is enabled
    pub identity_provider_configurations: Option<Vec<IdpConfig>>,

    /// The username to use if no authentication method is provided
    pub default_username: String,

    /// When an authentication method is provided, perform the authentication
    /// but always use the default username instead of the one provided by the
    /// authentication method
    pub force_default_username: bool,

    /// The DB parameters may be supplied on the command line
    pub db_params: DbParams,

    /// Whether to clear the database on start
    pub clear_db_on_start: bool,

    pub hostname: String,

    pub port: u16,

    pub http_params: HttpParams,

    /// The certificate used to verify the client TLS certificates
    /// used for authentication
    pub authority_cert_file: Option<X509>,
}

/// Represents the server parameters.
impl ServerParams {
    /// Tries to create a `ServerParams` instance from the given `ClapConfig`.
    ///
    /// # Arguments
    ///
    /// * `conf` - The `ClapConfig` object containing the configuration
    ///   parameters.
    ///
    /// # Returns
    ///
    /// Returns a `FResult` containing the `ServerParams` instance if
    /// successful, or an error if the conversion fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion from `ClapConfig` to `ServerParams`
    /// fails.
    pub fn try_from(conf: ClapConfig) -> FResult<Self> {
        let http_params = HttpParams::try_from(&conf.http)?;

        // Should we verify the client TLS certificates?
        let authority_cert_file = conf
            .http
            .authority_cert_file
            .map(|cert_file| {
                if http_params.is_running_https() {
                    Self::load_cert(&cert_file)
                } else {
                    server_bail!(
                        "The authority certificate file can only be used when the server is \
                         running in HTTPS mode"
                    )
                }
            })
            .transpose()?;

        Ok(Self {
            identity_provider_configurations: conf.auth.extract_idp_configs()?,
            db_params: conf.db.init()?,
            clear_db_on_start: conf.db.clear_database,
            hostname: conf.http.hostname,
            port: conf.http.port,
            http_params,
            default_username: conf.default_username,
            force_default_username: conf.force_default_username,
            authority_cert_file,
        })
    }

    /// Loads the certificate from the given file path.
    ///
    /// # Arguments
    ///
    /// * `authority_cert_file` - The path to the authority certificate file.
    ///
    /// # Returns
    ///
    /// Returns a `FResult` containing the loaded `X509` certificate if
    /// successful, or an error if the loading fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the certificate file cannot be read or if the
    /// parsing of the certificate fails.
    fn load_cert(authority_cert_file: &PathBuf) -> FResult<X509> {
        // Open and read the file into a byte vector
        let pem_bytes = std::fs::read(authority_cert_file)?;

        // Parse the byte vector as a X509 object
        let x509 = X509::from_pem(pem_bytes.as_slice())?;
        Ok(x509)
    }
}

impl fmt::Debug for ServerParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut x = f.debug_struct("");
        let x = x
            .field(
                "findex_server_url",
                &format!(
                    "http{}://{}:{}",
                    if self.http_params.is_running_https() {
                        "s"
                    } else {
                        ""
                    },
                    &self.hostname,
                    &self.port
                ),
            )
            .field("db_params", &self.db_params)
            .field("clear_db_on_start", &self.clear_db_on_start);
        let x = if let Some(identity_provider_configurations) =
            &self.identity_provider_configurations
        {
            x.field(
                "identity_provider_configurations",
                &identity_provider_configurations,
            )
        } else {
            x
        };
        let x = if let Some(verify_cert) = &self.authority_cert_file {
            x.field("verify_cert CN", verify_cert.subject_name())
        } else {
            x
        };
        let x = x
            .field("default_username", &self.default_username)
            .field("force_default_username", &self.force_default_username);
        let x = x.field("http_params", &self.http_params);
        x.finish()
    }
}

/// Creates a partial clone of the `ServerParams`
/// the `DbParams` and PKCS#12 information is not copied
/// since it may contain sensitive material
impl Clone for ServerParams {
    fn clone(&self) -> Self {
        warn!(
            "Cloning ServerParams, the `DbParams` and PKCS#12 information is not copied since it \
             may contain sensitive material and hence should be set manually"
        );
        Self {
            identity_provider_configurations: self.identity_provider_configurations.clone(),
            default_username: self.default_username.clone(),
            force_default_username: self.force_default_username,
            db_params: DbParams::default(),
            clear_db_on_start: self.clear_db_on_start,
            hostname: self.hostname.clone(),
            port: self.port,
            http_params: HttpParams::Http,
            authority_cert_file: self.authority_cert_file.clone(),
        }
    }
}
