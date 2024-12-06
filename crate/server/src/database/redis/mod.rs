// use async_trait::async_trait;

// use super::database_traits::DatabaseTraits;

mod datasets;
mod findex;
mod instance;
mod permissions;

// Word length is function of the serialization function provided when findex is instantiated
// In the (naïve) case of dummy_encode / dummy_decode as provided in findex benches,
// WORD_LENGTH = 1 + CHUNK_LENGTH = 1 + (8 * BLOCK_LENGTH) = 129 for a BLOCK_LENGTH set to 16.
pub(crate) const WORD_LENGTH: usize = 129;

pub(crate) use instance::Redis;

use crate::database::database_traits::DatabaseTraits;
impl DatabaseTraits for Redis<WORD_LENGTH> {}
