pub use error::{result::FindexClientResult, FindexClientError};
pub use findex_rest_client::{handle_error, FindexClient};

mod datasets;
mod error;
mod findex_rest_client;
mod permissions;
pub mod reexport {
    pub use cosmian_findex_config;
    pub use cosmian_http_client;
}
