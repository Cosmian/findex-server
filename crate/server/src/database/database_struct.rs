use cosmian_findex::{Address, ADDRESS_LENGTH};

use super::database_traits::FindexMemoryTrait;

// Word length is function of the serialization function provided when findex is instantiated
// In the (naïve) case of dummy_encode / dummy_decode as provided in findex benches,
// WORD_LENGTH = 1 + CHUNK_LENGTH = 1 + (8 * BLOCK_LENGTH) = 129 for a BLOCK_LENGTH set to 16.
pub(crate) type FindexMemoryType<const WORD_LENGTH: usize> = dyn FindexMemoryTrait<
    Address = Address<ADDRESS_LENGTH>,
    Word = [u8; WORD_LENGTH],
    Error = dyn Send + Sync + std::error::Error,
>;

// pub struct FindexDatabase<const WORD_LENGTH: usize> {
//     pub memory: Box<FindexMemoryType<8>>,
// }
