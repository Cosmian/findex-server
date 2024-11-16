use std::fmt::Display;

use super::FindexConfigError;

pub(crate) type FindexConfigResult<R> = Result<R, FindexConfigError>;

#[allow(dead_code)]
pub(crate) trait ConfigResultHelper<T> {
    fn context(self, context: &str) -> FindexConfigResult<T>;
    fn with_context<D, O>(self, op: O) -> FindexConfigResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D;
}

impl<T, E> ConfigResultHelper<T> for Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, context: &str) -> FindexConfigResult<T> {
        self.map_err(|e| FindexConfigError::Default(format!("{context}: {e}")))
    }

    fn with_context<D, O>(self, op: O) -> FindexConfigResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.map_err(|e| FindexConfigError::Default(format!("{}: {e}", op())))
    }
}

impl<T> ConfigResultHelper<T> for Option<T> {
    fn context(self, context: &str) -> FindexConfigResult<T> {
        self.ok_or_else(|| FindexConfigError::Default(context.to_string()))
    }

    fn with_context<D, O>(self, op: O) -> FindexConfigResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.ok_or_else(|| FindexConfigError::Default(format!("{}", op())))
    }
}
