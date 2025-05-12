use cosmian_findex::{RedisMemoryError, SqliteMemoryError};
use thiserror::Error;

/// Wraps memory errors from different findex memories
#[derive(Error, Debug)]
pub(crate) enum DatabaseError {
    #[error("Redis error: {0}")]
    RedisFindexMemoryError(#[from] RedisMemoryError),

    #[error("SQLite error: {0}")]
    SqliteFindexMemoryError(#[from] SqliteMemoryError),

    #[error("Redis connection error: {0}")]
    RedisCoreError(#[from] redis::RedisError),

    #[error("SQLite connection error: {0}")]
    AsyncSqliteCoreError(#[from] async_sqlite::Error),
    // maps to the cases when the server expects a specific type of data and the database returns
    // something else that's not convertible to the expected type
    #[error("Database returned invalid data : {0}")]
    InvalidDatabaseResponse(String),
    #[error("Invalid database type: {0} expected, {1} passed")]
    InvalidDatabaseType(String, String),
    #[error("Invalid database url: {0}")]
    InvalidDatabaseUrl(String),
}
