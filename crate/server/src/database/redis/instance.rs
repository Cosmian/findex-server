use cosmian_findex::{Address, RedisMemory, ADDRESS_LENGTH};

type RedisAdrType = Address<ADDRESS_LENGTH>;
type RedisWordType<const WORD_LENGTH: usize> = [u8; WORD_LENGTH];
pub(crate) struct ServerRedis<const WORD_LENGTH: usize> {
    pub(crate) memory: RedisMemory<RedisAdrType, RedisWordType<WORD_LENGTH>>,
}
