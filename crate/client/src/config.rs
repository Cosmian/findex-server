use std::path::PathBuf;

use cosmian_http_client::HttpClientConfig;
use serde::{Deserialize, Serialize};

use crate::{ClientError, ClientResult};
use cosmian_config_utils::{location, ConfigUtils};

pub const FINDEX_CLI_CONF_ENV: &str = "FINDEX_CLI_CONF";
pub(crate) const FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH: &str = "/etc/cosmian/findex.toml";
pub(crate) const FINDEX_CLI_CONF_PATH: &str = ".cosmian/findex.toml";

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct RestClientConfig {
    pub http_config: HttpClientConfig,
}

impl ConfigUtils for RestClientConfig {}

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
    /// * `conf_path` - The path to the configuration file
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

    /// Load the configuration from the given path
    ///
    /// # Errors
    /// Return an error if the configuration file is not found or if the
    /// configuration is invalid
    pub fn load(conf_path: Option<PathBuf>) -> Result<Self, ClientError> {
        let conf_path_buf = Self::location(conf_path)?;
        println!("Loading configuration from: {conf_path_buf:?}");

        Ok(Self::from_toml(conf_path_buf.to_str().ok_or_else(
            || {
                ClientError::Default(
                    "Unable to convert the configuration path to a string".to_owned(),
                )
            },
        )?)?)
    }

    /// Save the configuration to the given path
    ///
    /// # Errors
    /// Return an error if the configuration file is not found or if the
    /// configuration is invalid
    pub fn save(&self, conf_path: Option<PathBuf>) -> Result<(), ClientError> {
        let conf_path_buf = Self::location(conf_path)?;

        self.to_toml(conf_path_buf.to_str().ok_or_else(|| {
            ClientError::Default("Unable to convert the configuration path to a string".to_owned())
        })?)?;
        println!("Saving configuration to: {conf_path_buf:?}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use cosmian_config_utils::get_default_conf_path;
    use cosmian_logger::log_init;

    use super::{RestClientConfig, FINDEX_CLI_CONF_ENV};
    use crate::{config::FINDEX_CLI_CONF_PATH, ClientResult};

    #[test]
    #[allow(clippy::panic_in_result_fn, clippy::unwrap_used)] // this is a test
    pub(crate) fn test_load() -> ClientResult<()> {
        log_init(None);
        // valid conf
        unsafe {
            env::set_var(FINDEX_CLI_CONF_ENV, "../../test_data/configs/findex.toml");
        }
        RestClientConfig::load(None)?;

        // another valid conf
        unsafe {
            env::set_var(
                FINDEX_CLI_CONF_ENV,
                "../../test_data/configs/findex_partial.toml",
            );
        }
        RestClientConfig::load(None)?;

        // Default conf file
        unsafe {
            env::remove_var(FINDEX_CLI_CONF_ENV);
        }
        drop(fs::remove_file(get_default_conf_path(
            FINDEX_CLI_CONF_PATH,
        )?));
        RestClientConfig::load(None)?;
        assert!(get_default_conf_path(FINDEX_CLI_CONF_PATH)?.exists());

        // invalid conf
        unsafe {
            env::set_var(
                FINDEX_CLI_CONF_ENV,
                "../../test_data/configs/findex.bad.toml",
            );
        }
        let e = RestClientConfig::load(None).err().unwrap().to_string();
        assert!(e.contains("missing field `server_url`"));

        // with a file
        unsafe {
            env::remove_var(FINDEX_CLI_CONF_ENV);
        }
        let conf_path =
            RestClientConfig::location(Some(PathBuf::from("../../test_data/configs/findex.toml")))?;
        RestClientConfig::load(Some(conf_path))?;
        Ok(())
    }
}
