use std::str::Utf8Error;

#[cfg(test)]
use assert_cmd::cargo::CargoError;
use cosmian_config_utils::ConfigUtilsError;
use cosmian_findex_client::{
    ClientError,
    reexport::{cosmian_findex_structs::StructsError, cosmian_http_client::HttpClientError},
};
use cosmian_kms_cli::{
    error::KmsCliError,
    reexport::{cosmian_kmip::KmipError, cosmian_kms_client::KmsClientError},
};
use cosmian_sse_memories::{self, ADDRESS_LENGTH, Address};
use thiserror::Error;

pub mod result;

// Each error type must have a corresponding HTTP status code (see
// `kmip_endpoint.rs`)
#[derive(Error, Debug)]
pub enum FindexCliError {
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[cfg(test)]
    #[error(transparent)]
    CargoError(#[from] CargoError),
    #[error("{0}")]
    Configuration(String),
    #[error("Conversion error: {0}")]
    Conversion(String),
    #[error(transparent)]
    ConfigUtilsError(#[from] ConfigUtilsError),
    #[error(transparent)]
    CsvError(#[from] csv::Error),
    #[error("{0}")]
    Default(String),
    #[error(transparent)]
    Findex(#[from] cosmian_findex::Error<Address<ADDRESS_LENGTH>>),
    #[error(transparent)]
    FindexClientConfig(#[from] ClientError),
    #[error(transparent)]
    HttpClientError(#[from] HttpClientError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    KmipError(#[from] KmipError),
    #[error(transparent)]
    KmsClientError(#[from] KmsClientError),
    #[error(transparent)]
    KmsCliError(#[from] KmsCliError),
    #[error(transparent)]
    StructsError(#[from] StructsError),
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
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
        $crate::error::FindexCliError::Default(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::error::FindexCliError::Default($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::FindexCliError::Default(::core::format_args!($fmt, $($arg)*).to_string())
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
mod tests {

    use crate::error::result::FindexCliResult;

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

    fn bail() -> FindexCliResult<()> {
        let var = 43;
        if true {
            cli_bail!("interpolate {var}");
        }
        Ok(())
    }

    fn ensure() -> FindexCliResult<()> {
        let var = 44;
        cli_ensure!(false, "interpolate {var}");
        Ok(())
    }
}
