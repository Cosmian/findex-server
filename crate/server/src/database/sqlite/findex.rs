use cosmian_findex_memories::SqliteMemoryError;
use cosmian_findex_memories::reexport::cosmian_findex::{Address, MemoryADT};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;

use super::Sqlite;

impl<const WORD_LENGTH: usize> MemoryADT for Sqlite<WORD_LENGTH> {
    type Address = Address<SERVER_ADDRESS_LENGTH>;
    type Error = SqliteMemoryError;
    type Word = [u8; WORD_LENGTH];

    async fn batch_read(
        &self,
        addresses: Vec<Address<SERVER_ADDRESS_LENGTH>>,
    ) -> Result<Vec<Option<Self::Word>>, Self::Error> {
        self.memory.batch_read(addresses).await
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, Self::Error> {
        self.memory.guarded_write(guard, bindings).await
    }
}
