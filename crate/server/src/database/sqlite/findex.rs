use cosmian_findex::{Address, MemoryADT, SqliteMemoryError};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;

use super::_Sqlite;

impl<const WORD_LENGTH: usize> MemoryADT for _Sqlite<WORD_LENGTH> {
    type Address = Address<SERVER_ADDRESS_LENGTH>;
    type Word = [u8; WORD_LENGTH];
    type Error = SqliteMemoryError;

    async fn batch_read(
        &self,
        addresses: Vec<Address<SERVER_ADDRESS_LENGTH>>,
    ) -> Result<Vec<Option<Self::Word>>, SqliteMemoryError> {
        self.memory.batch_read(addresses).await
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, SqliteMemoryError> {
        self.memory.guarded_write(guard, bindings).await
    }
}
