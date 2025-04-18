use cosmian_findex::{Address, MemoryADT, RedisMemoryError as MemoryError};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;

use super::Redis;

impl<const WORD_LENGTH: usize> MemoryADT for Redis<WORD_LENGTH> {
    type Address = Address<SERVER_ADDRESS_LENGTH>;
    type Word = [u8; WORD_LENGTH];
    type Error = MemoryError;

    async fn batch_read(
        &self,
        addresses: Vec<Address<SERVER_ADDRESS_LENGTH>>,
    ) -> Result<Vec<Option<Self::Word>>, MemoryError> {
        self.memory.batch_read(addresses).await
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, MemoryError> {
        self.memory.guarded_write(guard, bindings).await
    }
}
