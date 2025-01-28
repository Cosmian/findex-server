use crate::error::result::FResult;
use cosmian_findex::{Address, MemoryADT, MemoryError, RedisMemory, ADDRESS_LENGTH};
use redis::aio::ConnectionManager;
use tracing::info;

pub(crate) struct Redis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) manager: ConnectionManager,
}

impl<const WORD_LENGTH: usize> Redis<WORD_LENGTH> {
    #[allow(dependency_on_unit_never_type_fallback)] // TODO: should be fixed before rust compiler update
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let mut manager = client.get_connection_manager().await?;
        let memory = RedisMemory::connect_with_manager(manager.clone()).await?;
        if clear_database {
            // TODO: unit test this
            info!("Warning: irreversible operation: clearing the database");
            redis::cmd("FLUSHDB").query_async(&mut manager).await?;
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
