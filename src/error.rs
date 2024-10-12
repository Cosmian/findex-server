use std::{array::TryFromSliceError, sync::mpsc::SendError};

use actix_web::{dev::ServerHandle, error::QueryPayloadError};
use cloudproof::reexport::crypto_core::CryptoCoreError;
use cloudproof_findex::implementations::redis::FindexRedisError;
use redis::ErrorKind;
use thiserror::Error;
use x509_parser::prelude::{PEMError, X509Error};

// Each error type must have a corresponding HTTP status code (see `kmip_endpoint.rs`)
#[derive(Error, Debug, Clone)]
pub enum FindexServerError {
    // When a conversion from/to bytes
    #[error("Conversion Error: {0}")]
    ConversionError(String),

    // Missing arguments in the request
    #[error("Invalid Request: {0}")]
    InvalidRequest(String),

    // Any errors related to a bad behavior of the DB but not related to the user input
    #[error("Database Error: {0}")]
    DatabaseError(String),

    // Any errors related to a bad behavior of the server but not related to the user input
    #[error("Unexpected server error: {0}")]
    ServerError(String),

    // Any actions of the user which is not allowed
    #[error("REST client connection error: {0}")]
    ClientConnectionError(String),

    // Any actions of the user which is not allowed
    #[error("Access denied: {0}")]
    Unauthorized(String),

    // A failure originating from one of the cryptographic algorithms
    #[error("Cryptographic error: {0}")]
    CryptographicError(String),

    // Error related to X509 Certificate
    #[error("Certificate error: {0}")]
    Certificate(String),

    #[error("Redis Error: {0}")]
    Redis(String),

    #[error("Findex Error: {0}")]
    Findex(String),

    #[error("Invalid URL: {0}")]
    UrlError(String),
}

impl From<x509_parser::nom::Err<X509Error>> for FindexServerError {
    fn from(e: x509_parser::nom::Err<X509Error>) -> Self {
        Self::Certificate(e.to_string())
    }
}

impl From<X509Error> for FindexServerError {
    fn from(e: X509Error) -> Self {
        Self::Certificate(e.to_string())
    }
}

impl From<&X509Error> for FindexServerError {
    fn from(e: &X509Error) -> Self {
        Self::Certificate(e.to_string())
    }
}

impl From<x509_parser::nom::Err<PEMError>> for FindexServerError {
    fn from(e: x509_parser::nom::Err<PEMError>) -> Self {
        Self::Certificate(e.to_string())
    }
}

impl From<CryptoCoreError> for FindexServerError {
    fn from(e: CryptoCoreError) -> Self {
        Self::CryptographicError(e.to_string())
    }
}

impl From<FindexRedisError> for FindexServerError {
    fn from(e: FindexRedisError) -> Self {
        Self::Findex(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for FindexServerError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::ConversionError(e.to_string())
    }
}

impl From<std::num::TryFromIntError> for FindexServerError {
    fn from(e: std::num::TryFromIntError) -> Self {
        Self::ConversionError(e.to_string())
    }
}

impl From<sqlx::Error> for FindexServerError {
    fn from(e: sqlx::Error) -> Self {
        Self::DatabaseError(e.to_string())
    }
}

impl From<std::io::Error> for FindexServerError {
    fn from(e: std::io::Error) -> Self {
        Self::ServerError(e.to_string())
    }
}

impl From<openssl::error::ErrorStack> for FindexServerError {
    fn from(e: openssl::error::ErrorStack) -> Self {
        Self::ServerError(format!("{e}. Details: {e:?}"))
    }
}

impl From<serde_json::Error> for FindexServerError {
    fn from(e: serde_json::Error) -> Self {
        Self::InvalidRequest(e.to_string())
    }
}

impl From<cloudproof::reexport::cover_crypt::Error> for FindexServerError {
    fn from(e: cloudproof::reexport::cover_crypt::Error) -> Self {
        Self::InvalidRequest(e.to_string())
    }
}

impl From<QueryPayloadError> for FindexServerError {
    fn from(e: QueryPayloadError) -> Self {
        Self::InvalidRequest(e.to_string())
    }
}

impl From<TryFromSliceError> for FindexServerError {
    fn from(e: TryFromSliceError) -> Self {
        Self::ConversionError(e.to_string())
    }
}

impl From<reqwest::Error> for FindexServerError {
    fn from(e: reqwest::Error) -> Self {
        Self::ClientConnectionError(format!("{e}: details: {e:?}"))
    }
}

impl From<SendError<ServerHandle>> for FindexServerError {
    fn from(e: SendError<ServerHandle>) -> Self {
        Self::ServerError(format!("Failed to send the server handle: {e}"))
    }
}

impl From<redis::RedisError> for FindexServerError {
    fn from(err: redis::RedisError) -> Self {
        Self::Redis(err.to_string())
    }
}

impl From<FindexServerError> for redis::RedisError {
    fn from(val: FindexServerError) -> Self {
        Self::from((
            ErrorKind::ClientError,
            "Findex Server Error",
            val.to_string(),
        ))
    }
}

impl From<url::ParseError> for FindexServerError {
    fn from(e: url::ParseError) -> Self {
        Self::UrlError(e.to_string())
    }
}

impl From<base64::DecodeError> for FindexServerError {
    fn from(e: base64::DecodeError) -> Self {
        Self::ConversionError(e.to_string())
    }
}

impl From<tracing::dispatcher::SetGlobalDefaultError> for FindexServerError {
    fn from(e: tracing::dispatcher::SetGlobalDefaultError) -> Self {
        Self::ServerError(e.to_string())
    }
}

/// Return early with an error if a condition is not satisfied.
///
/// This macro is equivalent to `if !$cond { return Err(From::from($err)); }`.
#[macro_export]
macro_rules! findex_server_ensure {
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            return ::core::result::Result::Err($crate::findex_server_error!($msg));
        }
    };
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return ::core::result::Result::Err($err);
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            return ::core::result::Result::Err($crate::findex_server_error!($fmt, $($arg)*));
        }
    };
}

/// Construct a server error from a string.
#[macro_export]
macro_rules! findex_server_error {
    ($msg:literal) => {
        $crate::error::FindexServerError::ServerError(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::error::FindexServerError::ServerError($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::FindexServerError::ServerError(::core::format_args!($fmt, $($arg)*).to_string())
    };
}

/// Return early with an error if a condition is not satisfied.
#[macro_export]
macro_rules! findex_server_bail {
    ($msg:literal) => {
        return ::core::result::Result::Err($crate::findex_server_error!($msg))
    };
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::core::result::Result::Err($crate::findex_server_error!($fmt, $($arg)*))
    };
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::FindexServerError;

    #[test]
    fn test_findex_server_error_interpolation() {
        let var = 42;
        let err = findex_server_error!("interpolate {var}");
        assert_eq!("Unexpected server error: interpolate 42", err.to_string());

        let err = bail();
        err.expect_err("Unexpected server error: interpolate 43");

        let err = ensure();
        err.expect_err("Unexpected server error: interpolate 44");
    }

    fn bail() -> Result<(), FindexServerError> {
        let var = 43;
        if true {
            findex_server_bail!("interpolate {var}");
        }
        Ok(())
    }

    fn ensure() -> Result<(), FindexServerError> {
        let var = 44;
        findex_server_ensure!(false, "interpolate {var}");
        Ok(())
    }
}
