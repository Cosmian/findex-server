use cloudproof_findex::reexport::cosmian_crypto_core::CryptoCoreError;
use thiserror::Error;

pub(crate) mod result;

#[derive(Error, Clone, Debug)]
pub enum StructsError {
    #[error(transparent)]
    Uuid(#[from] uuid::Error),

    #[error("{0}")]
    Default(String),

    #[error("Indexing slicing: {0}")]
    IndexingSlicing(String),

    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),

    #[error("Crypto core error: {0}")]
    Crypto(String),
}

impl From<CryptoCoreError> for StructsError {
    fn from(err: CryptoCoreError) -> Self {
        Self::Crypto(err.to_string())
    }
}

/// Construct a server error from a string.
#[macro_export]
macro_rules! structs_error {
    ($msg:literal) => {
        $crate::error::StructsError::Default(::core::format_args!($msg).to_string())
    };
    ($err:expr $(,)?) => ({
        $crate::error::StructsError::Default($err.to_string())
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::StructsError::Default(::core::format_args!($fmt, $($arg)*).to_string())
    };
}

/// Return early with an error if a condition is not satisfied.
#[macro_export]
macro_rules! structs_bail {
    ($msg:literal) => {
        return ::core::result::Result::Err($crate::structs_error!($msg))
    };
    ($err:expr $(,)?) => {
        return ::core::result::Result::Err($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::core::result::Result::Err($crate::structs_error!($fmt, $($arg)*))
    };
}
