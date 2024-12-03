use std::fmt::Display;

use super::StructsError;

pub(crate) type StructsResult<R> = Result<R, StructsError>;

#[allow(dead_code)]
pub(crate) trait StructsResultHelper<T> {
    fn context(self, context: &str) -> StructsResult<T>;
    fn with_context<D, O>(self, op: O) -> StructsResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D;
}

impl<T, E> StructsResultHelper<T> for Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, context: &str) -> StructsResult<T> {
        self.map_err(|e| StructsError::Default(format!("{context}: {e}")))
    }

    fn with_context<D, O>(self, op: O) -> StructsResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.map_err(|e| StructsError::Default(format!("{}: {e}", op())))
    }
}

impl<T> StructsResultHelper<T> for Option<T> {
    fn context(self, context: &str) -> StructsResult<T> {
        self.ok_or_else(|| StructsError::Default(context.to_owned()))
    }

    fn with_context<D, O>(self, op: O) -> StructsResult<T>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        self.ok_or_else(|| StructsError::Default(format!("{}", op())))
    }
}
