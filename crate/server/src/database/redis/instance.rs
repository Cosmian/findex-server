use crate::error::result::FResult;
use cosmian_findex::{Address, MemoryADT, MemoryError, RedisMemory, ADDRESS_LENGTH};
use redis::aio::ConnectionManager;
use tracing::info;
type RedisAdrType = Address<ADDRESS_LENGTH>;
type RedisWordType<const WORD_LENGTH: usize> = [u8; WORD_LENGTH];

pub(crate) struct Redis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<RedisAdrType, RedisWordType<WORD_LENGTH>>,
    pub(crate) manager: ConnectionManager,
}

impl<const WORD_LENGTH: usize> Redis<WORD_LENGTH> {
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = client.get_connection_manager().await?;
        let memory = RedisMemory::connect_with_manager(manager.clone()).await?;
        if clear_database {
            info!("Warning: irreversible operation: clearing the database");
            // TODO: fix this
            // memory.clear_redis_db().await?;
        }
        Ok(Self { memory, manager })
    }
}

impl<const WORD_LENGTH: usize> MemoryADT for Redis<WORD_LENGTH> {
    type Address = Address<ADDRESS_LENGTH>;
    type Word = [u8; WORD_LENGTH];
    type Error = MemoryError;

    async fn batch_read(
        &self,
        a: Vec<Address<ADDRESS_LENGTH>>,
    ) -> Result<Vec<Option<Self::Word>>, MemoryError> {
        self.memory.batch_read(a).await
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        tasks: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, MemoryError> {
        self.memory.guarded_write(guard, tasks).await
    }
}
