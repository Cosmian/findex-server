use std::str::Utf8Error;

#[cfg(test)]
use assert_cmd::cargo::CargoError;
use cloudproof_findex::{
    db_interfaces::DbInterfaceError,
    reexport::{cosmian_crypto_core::CryptoCoreError, cosmian_findex},
};
use cosmian_config_utils::ConfigUtilsError;
use cosmian_findex_client::{
    reexport::{cosmian_findex_config::FindexConfigError, cosmian_http_client::HttpClientError},
    FindexClientError,
};
use hex::FromHexError;
use thiserror::Error;

pub mod result;

// Each error type must have a corresponding HTTP status code
#[derive(Error, Debug)]
pub enum CliError {
    #[error(transparent)]
    ConfigError(#[from] FindexConfigError),
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
    DbInterfaceError(#[from] DbInterfaceError),
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
    ErrorDbInterfaceError(#[from] cosmian_findex::Error<DbInterfaceError>),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
    // Other errors
    #[error("{0}")]
    Default(String),
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
