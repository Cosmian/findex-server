use crate::error::{result::FResult, server::FindexServerError};
use cosmian_findex::{Address, RedisMemory};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;
use redis::aio::ConnectionManager;
use tracing::info;

pub(crate) struct Redis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<Address<SERVER_ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) manager: ConnectionManager,
}

impl<const WORD_LENGTH: usize> Redis<WORD_LENGTH> {
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let mut manager = client.get_connection_manager().await?;

        if clear_database {
            info!("Warning: proceeding to clear the database, this operation is irreversible.");
            let deletion_result: String = redis::cmd("FLUSHDB").query_async(&mut manager).await?;
            if deletion_result.as_str() == "OK" {
                info!("Database cleared");
            } else {
                return Err(FindexServerError::DatabaseError(
                    "Database not cleared, Redis DB returned {deletion_result}".to_owned(),
                ));
            }
        }

        let memory = RedisMemory::connect_with_manager(manager.clone()).await?;

        Ok(Self { memory, manager })
    }
}
