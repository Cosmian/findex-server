mod database_traits;
pub(crate) mod redis;

pub(crate) use database_traits::DatabaseTraits;
pub(crate) use database_traits::FindexMemoryTrait;
pub(crate) use redis::ServerRedis;

#[cfg(test)]
mod tests;
