use crate::database::{database_traits::InstantializationTrait, findex_database::FDBResult};
use async_trait::async_trait;
use cosmian_findex::{Address, RedisMemory};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;
use redis::aio::ConnectionManager;
use tracing::info;

pub(crate) struct Redis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<Address<SERVER_ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) manager: ConnectionManager,
}

#[async_trait]
impl<const WORD_LENGTH: usize> InstantializationTrait for Redis<WORD_LENGTH> {
    async fn instantiate(db_url: &str, clear_database: bool) -> FDBResult<Self> {
        let client = redis::Client::open(db_url)?;
        let mut manager = client.get_connection_manager().await?;

        if clear_database {
            info!("Warning: proceeding to clear the database, this operation is irreversible.");
            let deletion_result: String = redis::cmd("FLUSHDB").query_async(&mut manager).await?;
            if deletion_result.as_str() == "OK" {
                info!("Database cleared");
            } else {
                return Err(crate::database::DatabaseError::RedisCoreError(
                    redis::RedisError::from((
                        redis::ErrorKind::ResponseError,
                        "Failed to clear the database",
                        deletion_result,
                    )),
                ));
            }
        }

        let memory = RedisMemory::connect_with_manager(manager.clone()).await?;

        Ok(Self { memory, manager })
    }
}
