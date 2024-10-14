use super::error::LoginError;

pub(crate) type LoginResult<R> = Result<R, LoginError>;
