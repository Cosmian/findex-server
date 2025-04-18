use crate::{
    database::database_traits::InstantializationTrait,
    error::{result::FResult, server::ServerError},
};
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
    async fn instantiate(
        redis_url: &str,
        table_name: Option<&str>,
        clear_database: bool,
    ) -> FResult<Self> {
        if table_name.is_some() {
            return Err(ServerError::DatabaseError(
                "Table name is not supported in Redis".to_string(),
            ));
        }
        let client = redis::Client::open(redis_url)?;
        let mut manager = client.get_connection_manager().await?;

        if clear_database {
            info!("Warning: proceeding to clear the database, this operation is irreversible.");
            let deletion_result: String = redis::cmd("FLUSHDB").query_async(&mut manager).await?;
            if deletion_result.as_str() == "OK" {
                info!("Database cleared");
            } else {
                return Err(ServerError::DatabaseError(format!(
                    "Database not cleared, Redis DB returned {deletion_result}"
                )));
            }
        }

        let memory = RedisMemory::connect_with_manager(manager.clone()).await?;

        Ok(Self { memory, manager })
    }
}
