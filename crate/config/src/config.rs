use std::{
    env,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use cosmian_http_client::HttpClientConfig;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use tracing::info;
use tracing::trace;

#[cfg(target_os = "linux")]
use crate::config_bail;
use crate::error::{result::ConfigResultHelper, FindexConfigError};

/// Returns the path to the current user's home folder.
///
/// On Linux and macOS, the home folder is typically located at
/// `/home/<username>` or `/Users/<username>`, respectively. On Windows, the
/// home folder is typically located at `C:\Users\<username>`. However, the
/// location of the home folder can be changed by the user or by system
/// administrators, so it's important to check for the existence of the
/// appropriate environment variables.
///
/// Returns `None` if the home folder cannot be determined.
fn get_home_folder() -> Option<PathBuf> {
    // Check for the existence of the HOME environment variable on Linux and macOS
    if let Some(home) = env::var_os("HOME") {
        return Some(PathBuf::from(home));
    }
    // Check for the existence of the USERPROFILE environment variable on Windows
    else if let Some(profile) = env::var_os("USERPROFILE") {
        return Some(PathBuf::from(profile));
    }
    // Check for the existence of the HOMEDRIVE and HOMEPATH environment variables on Windows
    else if let (Some(hdrive), Some(hpath)) = (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH"))
    {
        return Some(PathBuf::from(hdrive).join(hpath));
    }
    // If none of the above environment variables exist, the home folder cannot be
    // determined
    None
}

/// Returns the default configuration path
///  or an error if the path cannot be determined
fn get_default_conf_path() -> Result<PathBuf, FindexConfigError> {
    get_home_folder()
        .ok_or_else(|| {
            FindexConfigError::NotSupported("unable to determine the home folder".to_owned())
        })
        .map(|home| home.join(".cosmian/findex.json"))
}

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
#[cfg(target_os = "linux")]
pub(crate) const FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH: &str = "/etc/cosmian/findex.json";

impl FindexClientConfig {
    pub fn location(conf: Option<PathBuf>) -> Result<PathBuf, FindexConfigError> {
        trace!("Getting configuration file location");
        // Obtain the configuration file path from:
        // - the `--conf` arg
        // - the environment variable corresponding to `FINDEX_CLI_CONF_ENV`
        // - default to a pre-determined path
        if let Some(conf_path) = conf {
            if !conf_path.exists() {
                return Err(FindexConfigError::NotSupported(format!(
                    "Configuration file {conf_path:?} from CLI arg does not exist"
                )));
            }
            return Ok(conf_path);
        } else if let Ok(conf_path) = env::var(FINDEX_CLI_CONF_ENV).map(PathBuf::from) {
            // Error if the specified file does not exist
            if !conf_path.exists() {
                return Err(FindexConfigError::NotSupported(format!(
                    "Configuration file {conf_path:?} specified in {FINDEX_CLI_CONF_ENV} \
                     environment variable does not exist"
                )));
            }
            return Ok(conf_path);
        }

        let user_conf_path = get_default_conf_path();
        trace!("User conf path is at: {user_conf_path:?}");

        #[cfg(not(target_os = "linux"))]
        return user_conf_path;

        #[cfg(target_os = "linux")]
        match user_conf_path {
            Err(_) => {
                // no user home, this may be the system attempting a load
                let default_system_path = PathBuf::from(FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH);
                if default_system_path.exists() {
                    info!(
                        "No active user, using configuration at \
                         {FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH}"
                    );
                    return Ok(default_system_path);
                }
                config_bail!(
                    "no configuration found at {FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH}, and no \
                     current user, bailing out"
                );
            }
            Ok(user_conf) => {
                // the user home exists, if there is no conf file, check
                // /etc/cosmian/findex.json
                if !user_conf.exists() {
                    let default_system_path = PathBuf::from(FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH);
                    if default_system_path.exists() {
                        info!(
                            "Linux user conf path is at: {user_conf:?} but is empty, using \
                             {FINDEX_CLI_CONF_DEFAULT_SYSTEM_PATH} instead"
                        );
                        return Ok(default_system_path);
                    }
                    info!(
                        "Linux user conf path is at: {user_conf:?} and will be initialized with a \
                         default value"
                    );
                }
                Ok(user_conf)
            }
        }
    }

    pub fn save(&self, conf_path: &PathBuf) -> Result<(), FindexConfigError> {
        fs::write(
            conf_path,
            serde_json::to_string_pretty(&self)
                .with_context(|| format!("Unable to serialize default configuration {self:?}"))?,
        )
        .with_context(|| {
            format!("Unable to write default configuration to file {conf_path:?}\n{self:?}")
        })?;

        Ok(())
    }

    pub fn load(conf_path: &PathBuf) -> Result<Self, FindexConfigError> {
        // Deserialize the configuration from the file, or create a default
        // configuration if none exists
        let conf = if conf_path.exists() {
            // Configuration file exists, read and deserialize it
            let file = File::open(conf_path)
                .with_context(|| format!("Unable to read configuration file {conf_path:?}"))?;
            serde_json::from_reader(BufReader::new(file))
                .with_context(|| format!("Error while parsing configuration file {conf_path:?}"))?
        } else {
            // Configuration file doesn't exist, create it with default values and serialize
            // it
            let parent = conf_path
                .parent()
                .with_context(|| format!("Unable to get parent directory of {conf_path:?}"))?;
            fs::create_dir_all(parent).with_context(|| {
                format!("Unable to create directory for configuration file {parent:?}")
            })?;

            let default_conf = Self::default();
            default_conf.save(conf_path)?;
            default_conf
        };

        Ok(conf)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use cosmian_logger::log_init;

    use super::{get_default_conf_path, FindexClientConfig, FINDEX_CLI_CONF_ENV};

    #[test]
    pub(crate) fn test_load() {
        log_init(None);
        // valid conf
        unsafe {
            env::set_var(FINDEX_CLI_CONF_ENV, "test_data/configs/findex.json");
        }
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());

        // another valid conf
        unsafe {
            env::set_var(FINDEX_CLI_CONF_ENV, "test_data/configs/findex_partial.json");
        }
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());

        // Default conf file
        unsafe {
            env::remove_var(FINDEX_CLI_CONF_ENV);
        }
        let _ = fs::remove_file(get_default_conf_path().unwrap());
        let conf_path = FindexClientConfig::location(None).unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());
        assert!(get_default_conf_path().unwrap().exists());

        // invalid conf
        unsafe {
            env::set_var(FINDEX_CLI_CONF_ENV, "test_data/configs/findex.bad.json");
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
        let conf_path =
            FindexClientConfig::location(Some(PathBuf::from("test_data/configs/findex.json")))
                .unwrap();
        assert!(FindexClientConfig::load(&conf_path).is_ok());
    }
}
