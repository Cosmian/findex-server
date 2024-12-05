use std::fmt::Display;

use super::FindexClientError;

pub type FindexClientResult<R> = Result<R, FindexClientError>;

pub(crate) trait FindexRestClientResultHelper<T> {
    fn with_context<D, O>(self, op: O) -> FindexClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D;
}

impl<T, E> FindexRestClientResultHelper<T> for Result<T, E>
where
    E: std::error::Error,
{
    fn with_context<D, O>(self, op: O) -> FindexClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.map_err(|e| FindexClientError::Default(format!("{}: {e}", op())))
    }
}

impl<T> FindexRestClientResultHelper<T> for Option<T> {
    fn with_context<D, O>(self, op: O) -> FindexClientResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.ok_or_else(|| FindexClientError::Default(format!("{}", op())))
    }
}
