use crate::error::server::ServerError;

pub type FResult<R> = Result<R, ServerError>;

/// A helper trait for `FResult` that provides additional methods for error
/// handling.
pub trait FResultHelper<T> {
    /// Sets the context for the error.
    ///
    /// # Errors
    ///
    /// Returns a `FResult` with the specified context if the original result is
    /// an error.
    fn context(self, context: &str) -> FResult<T>;

    /// Sets the context for the error using a closure.
    ///
    /// # Errors
    ///
    /// Returns a `FResult` with the context returned by the closure if the
    /// original result is an error.
    fn with_context<O>(self, op: O) -> FResult<T>
    where
        O: FnOnce() -> String;
}

impl<T, E> FResultHelper<T> for std::result::Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, context: &str) -> FResult<T> {
        self.map_err(|e| ServerError::ServerError(format!("{context}: {e}")))
    }

    fn with_context<O>(self, op: O) -> FResult<T>
    where
        O: FnOnce() -> String,
    {
        self.map_err(|e| ServerError::ServerError(format!("{}: {e}", op())))
    }
}

impl<T> FResultHelper<T> for Option<T> {
    fn context(self, context: &str) -> FResult<T> {
        self.ok_or_else(|| ServerError::ServerError(context.to_owned()))
    }

    fn with_context<O>(self, op: O) -> FResult<T>
    where
        O: FnOnce() -> String,
    {
        self.ok_or_else(|| ServerError::ServerError(op()))
    }
}
