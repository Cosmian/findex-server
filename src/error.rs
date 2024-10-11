use crate::middlewares::LoginError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FindexServerError {
    #[error("Login error: {0}")]
    Login(#[from] LoginError),

    // Any errors related to a bad behavior of the server but not related to the user input
    #[error("Unexpected server error: {0}")]
    ServerError(String),
    // #[error("Configuration error: {0}")]
    // Config(String),

    // #[error("Unexpected error: {0}")]
    // Unexpected(String),
}

impl From<std::io::Error> for FindexServerError {
    fn from(e: std::io::Error) -> Self {
        Self::ServerError(e.to_string())
    }
}
