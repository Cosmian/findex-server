use cosmian_findex::MemoryError;
use std::sync::mpsc::SendError;

use actix_web::dev::ServerHandle;
use cosmian_findex_structs::StructsError;
use std::fmt::Debug;
use thiserror::Error;

// Each error type must have a corresponding HTTP status code
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
    #[error("Permission denied: {0}")]
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
    #[error(transparent)]
    StructsError(#[from] StructsError),
    #[error(transparent)]
    SendError(#[from] SendError<ServerHandle>),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    OpenSslError(#[from] openssl::error::ErrorStack),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
}

impl From<std::io::Error> for FindexServerError {
    fn from(e: std::io::Error) -> Self {
        Self::ServerError(e.to_string())
    }
}

impl From<redis::RedisError> for FindexServerError {
    fn from(err: redis::RedisError) -> Self {
        Self::Redis(err.to_string())
    }
}

impl From<MemoryError> for FindexServerError {
    fn from(e: MemoryError) -> Self {
        Self::DatabaseError(e.to_string())
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
        $crate::error::server::FindexServerError::ServerError(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::error::server::FindexServerError::ServerError($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::server::FindexServerError::ServerError(::core::format_args!($fmt, $($arg)*).to_string())
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

#[allow(clippy::expect_used)] // ok in tests
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
