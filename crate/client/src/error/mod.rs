use std::io;

use cosmian_findex_structs::StructsError;
use thiserror::Error;

pub(crate) mod result;

#[derive(Error, Debug)]
pub enum FindexClientError {
    #[error(transparent)]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Invalid conversion: {0}")]
    Conversion(String),

    #[error("{0}")]
    Default(String),

    #[error("Not Supported: {0}")]
    NotSupported(String),

    #[error(transparent)]
    PemError(#[from] pem::PemError),

    #[error("Ratls Error: {0}")]
    RatlsError(String),

    #[error("REST Request Failed: {0}")]
    RequestFailed(String),

    #[error("REST Response Conversion Failed: {0}")]
    ResponseFailed(String),

    #[error("TTLV Error: {0}")]
    TtlvError(String),

    #[error("Unexpected Error: {0}")]
    UnexpectedError(String),

    #[error(transparent)]
    UrlError(#[from] url::ParseError),

    #[error(transparent)]
    StructsError(#[from] StructsError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    ReqwestHeaderError(#[from] reqwest::header::InvalidHeaderValue),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    DerError(#[from] der::Error),
}

/// Construct a server error from a string.
#[macro_export]
macro_rules! findex_client_error {
    ($msg:literal) => {
        $crate::FindexClientError::Default(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::FindexClientError::Default($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::FindexClientError::Default(::core::format_args!($fmt, $($arg)*).to_string())
    };
}

/// Return early with an error if a condition is not satisfied.
#[macro_export]
macro_rules! findex_client_bail {
    ($msg:literal) => {
        return ::core::result::Result::Err($crate::findex_client_error!($msg))
    };
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::core::result::Result::Err($crate::findex_client_error!($fmt, $($arg)*))
    };
}
