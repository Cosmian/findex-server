use cosmian_findex::{Address, RedisMemory, ADDRESS_LENGTH};

use crate::database::database_traits::FindexMemoryTrait;

use super::{ServerRedis, WORD_LENGTH};

impl FindexMemoryTrait for ServerRedis<WORD_LENGTH> {
    type Memory = RedisMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>;

    fn get_memory(&self) -> &Self::Memory {
        &self.memory
    }
}
