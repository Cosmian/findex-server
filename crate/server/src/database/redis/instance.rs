use cosmian_findex::{Address, RedisMemory, ADDRESS_LENGTH};

pub(crate) struct ServerRedis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
}
