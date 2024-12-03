pub use config::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
pub use error::FindexConfigError;

mod config;
mod error;

pub mod reexport {
    pub use cosmian_config_utils;
}
