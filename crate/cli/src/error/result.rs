use std::fmt::Display;

use super::FindexCliError;

pub type FindexCliResult<R> = Result<R, FindexCliError>;

/// Trait for providing helper methods for `CosmianResult`.
pub trait FindexCliResultHelper<T> {
    /// Sets the context for the error.
    ///
    /// # Errors
    ///
    /// Returns a `CosmianResult` with the specified context.
    fn context(self, context: &str) -> FindexCliResult<T>;

    /// Sets the context for the error using a closure.
    ///
    /// # Errors
    ///
    /// Returns a `CosmianResult` with the context returned by the closure.
    fn with_context<D, O>(self, op: O) -> FindexCliResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D;
}

impl<T, E> FindexCliResultHelper<T> for std::result::Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, context: &str) -> FindexCliResult<T> {
        self.map_err(|e| FindexCliError::Default(format!("{context}: {e}")))
    }

    fn with_context<D, O>(self, op: O) -> FindexCliResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.map_err(|e| FindexCliError::Default(format!("{}: {e}", op())))
    }
}

impl<T> FindexCliResultHelper<T> for Option<T> {
    fn context(self, context: &str) -> FindexCliResult<T> {
        self.ok_or_else(|| FindexCliError::Default(context.to_owned()))
    }

    fn with_context<D, O>(self, op: O) -> FindexCliResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.ok_or_else(|| FindexCliError::Default(format!("{}", op())))
    }
}
