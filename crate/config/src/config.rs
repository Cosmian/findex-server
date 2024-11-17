use std::path::PathBuf;

use cosmian_config_utils::{location, ConfigUtils};
use cosmian_http_client::HttpClientConfig;
use serde::{Deserialize, Serialize};

use crate::error::result::FindexConfigResult;

/// This method is used to configure the FINDEX CLI by reading a JSON
/// configuration file.
///
/// The method looks for a JSON configuration file with the following structure:
///
/// ```json
/// {
/// "http_config": {
///     "accept_invalid_certs": false,
///     "server_url": "http://127.0.0.1:9998",
///     "access_token": "AA...AAA",
///     "database_secret": "BB...BBB",
///     "ssl_client_pkcs12_path": "/path/to/client.p12",
///     "ssl_client_pkcs12_password": "password"
///     }
/// }
/// ```
/// The path to the configuration file is specified through the
/// `FINDEX_CLI_CONF` environment variable. If the environment variable is not
/// set, a default path is used. If the configuration file does not exist at the
/// path, a new file is created with default values.
///
/// This function returns a FINDEX client configured according to the settings
/// specified in the configuration file.
pub const FINDEX_CLI_CONF_ENV: &str = "FINDEX_CLI_CONF";
pub(crate) const FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH: &str = "/etc/cosmian/findex.json";
pub(crate) const FINDEX_CLI_CONF_PATH: &str = ".cosmian/findex.json";

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct FindexClientConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf_path: Option<PathBuf>,
    pub http_config: HttpClientConfig,
}

impl Default for FindexClientConfig {
    fn default() -> Self {
        Self {
            conf_path: None,
            http_config: HttpClientConfig {
                server_url: "http://0.0.0.0:6668".to_owned(),
                ..HttpClientConfig::default()
            },
        }
    }
}

impl ConfigUtils for FindexClientConfig {}

impl FindexClientConfig {
    pub fn location(conf: Option<PathBuf>) -> FindexConfigResult<PathBuf> {
        Ok(location(
            conf,
            FINDEX_CLI_CONF_ENV,
            FINDEX_CLI_CONF_PATH,
            FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use cosmian_config_utils::{get_default_conf_path, ConfigUtils};
    use cosmian_logger::log_init;

    use super::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
    use crate::config::FINDEX_CLI_CONF_PATH;

    #[test]
    pub(crate) fn test_load() {
        log_init(None);
        // valid conf
        unsafe {
            env::set_var(FINDEX_CLI_CONF_ENV, "../../test_data/configs/findex.json");
        }
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());

        // another valid conf
        unsafe {
            env::set_var(
                FINDEX_CLI_CONF_ENV,
                "../../test_data/configs/findex_partial.json",
            );
        }
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());

        // Default conf file
        unsafe {
            env::remove_var(FINDEX_CLI_CONF_ENV);
        }
        let _ = fs::remove_file(get_default_conf_path(FINDEX_CLI_CONF_PATH).unwrap());
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());
        assert!(
            get_default_conf_path(FINDEX_CLI_CONF_PATH)
                .unwrap()
                .exists()
        );

        // invalid conf
        unsafe {
            env::set_var(
                FINDEX_CLI_CONF_ENV,
                "../../test_data/configs/findex.bad.json",
            );
        }
        let conf_path = FindexClientConfig::location(None).unwrap();
        let e = FindexClientConfig::load(&conf_path)
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains("missing field `server_url`"));

        // with a file
        unsafe {
            env::remove_var(FINDEX_CLI_CONF_ENV);
        }
        let conf_path = FindexClientConfig::location(Some(PathBuf::from(
            "../../test_data/configs/findex.json",
        )))
        .unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());
    }
}
