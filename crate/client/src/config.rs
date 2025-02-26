use std::path::PathBuf;

use cosmian_http_client::HttpClientConfig;
use serde::{Deserialize, Serialize};

use crate::ClientResult;
use cosmian_config_utils::location;

pub const FINDEX_CLI_CONF_ENV: &str = "FINDEX_CLI_CONF";
pub(crate) const FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH: &str = "/etc/cosmian/findex.toml";
pub(crate) const FINDEX_CLI_CONF_PATH: &str = ".cosmian/findex.toml";

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct RestClientConfig {
    pub http_config: HttpClientConfig,
}

impl Default for RestClientConfig {
    fn default() -> Self {
        Self {
            http_config: HttpClientConfig {
                server_url: "http://0.0.0.0:6668".to_owned(),
                ..HttpClientConfig::default()
            },
        }
    }
}

#[allow(clippy::print_stdout)] // expected behavior
impl RestClientConfig {
    /// Load the configuration from the given path
    ///
    /// # Arguments
    /// * `conf` - The path to the configuration file
    ///
    /// # Errors
    /// Return an error if the configuration file is not found or if the
    /// configuration is invalid
    pub fn location(conf: Option<PathBuf>) -> ClientResult<PathBuf> {
        Ok(location(
            conf,
            FINDEX_CLI_CONF_ENV,
            FINDEX_CLI_CONF_PATH,
            FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH,
        )?)
    }
}
