use super::error::LoginError;

pub type LoginResult<R> = Result<R, LoginError>;
