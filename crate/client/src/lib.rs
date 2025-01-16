use cosmian_findex::{Findex, Value};
use cosmian_findex_structs::WORD_LENGTH;

mod config;
mod datasets;
mod error;
mod permissions;
mod rest_client;

pub use config::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
pub use error::{result::FindexClientResult, FindexClientError};
pub use rest_client::{handle_error, FindexRestClient};

pub type InstantiatedFindex = Findex<WORD_LENGTH, Value, String, FindexRestClient>;

pub mod reexport {
    pub use cosmian_http_client;
}
