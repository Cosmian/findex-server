use lazy_static::lazy_static;
use rawsql::Loader;

mod database_trait;
mod redis;
mod sqlite;

pub(crate) type FServer = crate::core::FindexServer;

pub(crate) use database_trait::Database;
pub(crate) use redis::RedisWithFindex;
pub(crate) use redis::REDIS_WITH_FINDEX_MASTER_KEY_LENGTH;
pub(crate) use sqlite::SqlitePool;

const SQLITE_FILE_QUERIES: &str = include_str!("query.sql");

lazy_static! {
    static ref SQLITE_QUERIES: Loader = #[allow(clippy::expect_used)]
    Loader::get_queries_from(SQLITE_FILE_QUERIES)
        .expect("Can't parse the SQL file");
}

#[cfg(test)]
mod tests;
