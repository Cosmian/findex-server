mod database_trait;
mod redis;

pub(crate) use database_trait::Database;
pub(crate) use redis::Redis;

#[cfg(test)]
mod tests;
