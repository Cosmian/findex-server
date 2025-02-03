use std::str::Utf8Error;

#[cfg(test)]
use assert_cmd::cargo::CargoError;
use cosmian_config_utils::ConfigUtilsError;
use cosmian_crypto_core::CryptoCoreError;
use cosmian_findex::{Address, ADDRESS_LENGTH};
use cosmian_findex_client::{reexport::cosmian_http_client::HttpClientError, FindexClientError};
use hex::FromHexError;
use thiserror::Error;

pub mod result;

// Each error type must have a corresponding HTTP status code
#[derive(Error, Debug)]
pub enum CliError {
    #[error(transparent)]
    FindexError(#[from] cosmian_findex::Error<Address<ADDRESS_LENGTH>>),
    #[error(transparent)]
    ConfigUtilsError(#[from] ConfigUtilsError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    CsvError(#[from] csv::Error),
    #[error(transparent)]
    HttpClientError(#[from] HttpClientError),
    #[error(transparent)]
    CryptoCoreError(#[from] CryptoCoreError),
    #[error(transparent)]
    FindexClientError(#[from] FindexClientError),
    #[error(transparent)]
    FromHexError(#[from] FromHexError),
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[cfg(test)]
    #[error(transparent)]
    CargoError(#[from] CargoError),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    // Other errors
    #[error("{0}")]
    Default(String),
}

/// Construct a server error from a string.
#[macro_export]
macro_rules! cli_error {
    ($msg:literal) => {
        $crate::error::CliError::Default(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::error::CliError::Default($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::CliError::Default(::core::format_args!($fmt, $($arg)*).to_string())
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_cli_error_interpolation() {
        let var = 42;
        let err = cli_error!("interpolate {var}");
        assert_eq!("interpolate 42", err.to_string());
    }
}
