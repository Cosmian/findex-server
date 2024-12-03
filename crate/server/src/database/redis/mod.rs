// todo(manu): fix it
// #![allow(clippy::blocks_in_conditions)]
mod datasets;
mod findex;
mod permissions;
use super::DatabaseTraits;
use cosmian_findex::{Address, RedisMemory, ADDRESS_LENGTH};

// Word length is function of the serialization function provided when findex is instantiated
// In the (naïve) case of dummy_encode / dummy_decode as provided in findex benches,
// WORD_LENGTH = 1 + CHUNK_LENGTH = 1 + (8 * BLOCK_LENGTH) = 129 for a BLOCK_LENGTH set to 16.
pub(crate) const WORD_LENGTH: usize = 129;

pub(crate) struct ServerRedis<const WORD_LENGTH: usize> {
    memory: RedisMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
}

impl DatabaseTraits for ServerRedis<WORD_LENGTH> {}
