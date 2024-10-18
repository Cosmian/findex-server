use std::fmt::Display;

use crate::error::ClientError;

pub type RestClientResult<R> = Result<R, ClientError>;

pub trait ClientResultHelper<T> {
    /// Add a context to the error message
    /// # Errors
    /// It returns the error with the context added
    fn context(self, context: &str) -> RestClientResult<T>;

    /// Add a context to the error message
    /// # Errors
    /// It returns the error with the context added
    fn with_context<D, O>(self, op: O) -> RestClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D;
}

impl<T, E> ClientResultHelper<T> for std::result::Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, context: &str) -> RestClientResult<T> {
        self.map_err(|e| ClientError::Default(format!("{context}: {e}")))
    }

    fn with_context<D, O>(self, op: O) -> RestClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.map_err(|e| ClientError::Default(format!("{}: {e}", op())))
    }
}

impl<T> ClientResultHelper<T> for Option<T> {
    fn context(self, context: &str) -> RestClientResult<T> {
        self.ok_or_else(|| ClientError::Default(context.to_owned()))
    }

    fn with_context<D, O>(self, op: O) -> RestClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.ok_or_else(|| ClientError::Default(format!("{}", op())))
    }
}
