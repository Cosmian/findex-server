use std::{fmt::Display, path::PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};

const DEFAULT_PORT: u16 = 6668;
const DEFAULT_HOSTNAME: &str = "0.0.0.0";

#[derive(Args, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct HttpConfig {
    /// The Findex server port
    #[clap(long, env = "FINDEX_SERVER_PORT", default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// The Findex server hostname
    #[clap(long, env = "FINDEX_SERVER_HOSTNAME", default_value = DEFAULT_HOSTNAME)]
    pub hostname: String,

    /// The Findex server optional PKCS#12 Certificates and Key file. If
    /// provided, this will start the server in HTTPS mode.
    #[clap(long, env = "FINDEX_SERVER_HTTPS_P12_FILE")]
    pub https_p12_file: Option<PathBuf>,

    /// The password to open the PKCS#12 Certificates and Key file
    #[clap(long, env = "FINDEX_SERVER_HTTPS_P12_PASSWORD")]
    pub https_p12_password: Option<String>,

    /// The server optional authority X509 certificate in PEM format used to
    /// validate the client certificate presented for authentication.
    /// If provided, this will require clients to present a certificate signed
    /// by this authority for authentication. The server must run in TLS
    /// mode for this to be used.
    #[clap(long, env = "FINDEX_SERVER_AUTHORITY_CERT_FILE")]
    pub authority_cert_file: Option<PathBuf>,
}

impl Display for HttpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.https_p12_file.is_some() {
            write!(f, "https://{}:{}, ", self.hostname, self.port)?;
            write!(f, "Pkcs12 file: {:?}, ", self.https_p12_file.as_ref())?;
            if let Some(https_p12_password) = &self.https_p12_password {
                write!(f, "password: {}, ", https_p12_password.replace('.', "*"))?;
            }
            write!(
                f,
                "authority cert file: {:?}",
                self.authority_cert_file.as_ref()
            )
        } else {
            write!(f, "http://{}:{}", self.hostname, self.port)
        }
    }
}

impl std::fmt::Debug for HttpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self))
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            hostname: DEFAULT_HOSTNAME.to_owned(),
            https_p12_file: None,
            https_p12_password: None,
            authority_cert_file: None,
        }
    }
}
