pub use config::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
pub use error::{result::FindexClientResult, FindexClientError};
pub use rest_client::{handle_error, FindexRestClient};

mod config;
mod datasets;
mod error;
mod permissions;
mod rest_client;

pub mod reexport {
    pub use cosmian_config_utils;
    pub use cosmian_http_client;
}
