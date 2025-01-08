use cosmian_findex::{Findex, Value};
use cosmian_findex_structs::WORD_LENGTH;

mod config;
mod datasets;
mod error;
mod permissions;
mod rest_client;

pub use config::{FINDEX_CLI_CONF_ENV, FindexClientConfig};
pub use error::{FindexClientError, result::FindexClientResult};
pub use rest_client::{FindexRestClient, handle_error};

pub type InstantiatedFindex =
    Findex<{ WORD_LENGTH }, Value, std::convert::Infallible, FindexRestClient>;

pub mod reexport {
    pub use cosmian_http_client;
}
