use std::{array::TryFromSliceError, num::TryFromIntError, str::Utf8Error};

#[cfg(test)]
use assert_cmd::cargo::CargoError;
use cloudproof_findex::{
    db_interfaces::DbInterfaceError,
    reexport::{cosmian_crypto_core::CryptoCoreError, cosmian_findex},
};
use cosmian_rest_client::ClientError;
use hex::FromHexError;
use pem::PemError;
use thiserror::Error;

pub mod result;

// Each error type must have a corresponding HTTP status code
#[derive(Error, Debug)]
pub enum CliError {
    // When a user requests an endpoint which does not exist
    #[error("Not Supported route: {0}")]
    RouteNotFound(String),

    // When a user requests something not supported by the server
    #[error("Not Supported: {0}")]
    NotSupported(String),

    // When a user requests something which is a non-sense
    #[error("Inconsistent operation: {0}")]
    InconsistentOperation(String),

    // When a user requests an id which does not exist
    #[error("Item not found: {0}")]
    ItemNotFound(String),

    // Missing arguments in the request
    #[error("Invalid Request: {0}")]
    InvalidRequest(String),

    // Any errors related to a bad behavior of the server but not related to the user input
    #[error("Server error: {0}")]
    ServerError(String),

    // Any actions of the user which is not allowed
    #[error("Access denied: {0}")]
    Unauthorized(String),

    // A cryptographic error
    #[error("Cryptographic error: {0}")]
    Cryptographic(String),

    // Conversion errors
    #[error("Conversion error: {0}")]
    Conversion(String),

    // When the Findex server client returns an error
    #[error("{0}")]
    KmsClientError(String),

    // Other errors
    #[error("invalid options: {0}")]
    UserError(String),

    // Other errors
    #[error("{0}")]
    Default(String),

    // Url parsing errors
    #[error(transparent)]
    UrlParsing(#[from] url::ParseError),

    // When an error occurs fetching Gmail API
    #[error("Error interacting with Gmail API: {0}")]
    GmailApiError(String),
}

impl From<der::Error> for CliError {
    fn from(e: der::Error) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<TryFromSliceError> for CliError {
    fn from(e: TryFromSliceError) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::ServerError(e.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<Utf8Error> for CliError {
    fn from(e: Utf8Error) -> Self {
        Self::Default(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Default(e.to_string())
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        Self::Default(format!("{e}: Details: {e:?}"))
    }
}

impl From<TryFromIntError> for CliError {
    fn from(e: TryFromIntError) -> Self {
        Self::Default(format!("{e}: Details: {e:?}"))
    }
}

#[cfg(test)]
impl From<CargoError> for CliError {
    fn from(e: CargoError) -> Self {
        Self::Default(e.to_string())
    }
}

impl From<base64::DecodeError> for CliError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<FromHexError> for CliError {
    fn from(e: FromHexError) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<ClientError> for CliError {
    fn from(e: ClientError) -> Self {
        Self::KmsClientError(e.to_string())
    }
}

impl From<PemError> for CliError {
    fn from(e: PemError) -> Self {
        Self::Conversion(format!("PEM error: {e}"))
    }
}

impl From<std::fmt::Error> for CliError {
    fn from(e: std::fmt::Error) -> Self {
        Self::Default(e.to_string())
    }
}

impl From<CryptoCoreError> for CliError {
    fn from(e: CryptoCoreError) -> Self {
        Self::Cryptographic(e.to_string())
    }
}

impl From<cosmian_findex::Error<DbInterfaceError>> for CliError {
    fn from(e: cosmian_findex::Error<DbInterfaceError>) -> Self {
        Self::Cryptographic(e.to_string())
    }
}

impl From<DbInterfaceError> for CliError {
    fn from(e: DbInterfaceError) -> Self {
        Self::Cryptographic(e.to_string())
    }
}

impl From<csv::Error> for CliError {
    fn from(e: csv::Error) -> Self {
        Self::Conversion(e.to_string())
    }
}

/// Return early with an error if a condition is not satisfied.
///
/// This macro is equivalent to `if !$cond { return Err(From::from($err)); }`.
#[macro_export]
macro_rules! cli_ensure {
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            return ::core::result::Result::Err($crate::cli_error!($msg));
        }
    };
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return ::core::result::Result::Err($err);
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            return ::core::result::Result::Err($crate::cli_error!($fmt, $($arg)*));
        }
    };
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

/// Return early with an error if a condition is not satisfied.
#[macro_export]
macro_rules! cli_bail {
    ($msg:literal) => {
        return ::core::result::Result::Err($crate::cli_error!($msg))
    };
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::core::result::Result::Err($crate::cli_error!($fmt, $($arg)*))
    };
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {

    use crate::error::result::CliResult;

    #[test]
    fn test_cli_error_interpolation() {
        let var = 42;
        let err = cli_error!("interpolate {var}");
        assert_eq!("interpolate 42", err.to_string());

        let err = bail();
        assert_eq!("interpolate 43", err.unwrap_err().to_string());

        let err = ensure();
        assert_eq!("interpolate 44", err.unwrap_err().to_string());
    }

    fn bail() -> CliResult<()> {
        let var = 43;
        if true {
            cli_bail!("interpolate {var}");
        }
        Ok(())
    }

    fn ensure() -> CliResult<()> {
        let var = 44;
        cli_ensure!(false, "interpolate {var}");
        Ok(())
    }
}
