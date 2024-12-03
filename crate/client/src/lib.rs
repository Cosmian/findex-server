pub use error::{result::FindexClientResult, FindexClientError};
pub use rest_client::{handle_error, FindexRestClient};

mod datasets;
mod error;
mod permissions;
mod rest_client;
pub mod reexport {
    pub use cosmian_findex_config;
    pub use cosmian_http_client;
}
