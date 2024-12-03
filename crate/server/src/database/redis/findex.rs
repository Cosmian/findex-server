use cosmian_findex::{Address, RedisMemory, ADDRESS_LENGTH};

pub(crate) struct ServerRedis {
    inner: RedisMemory<Address<ADDRESS_LENGTH>, 8>,
}
