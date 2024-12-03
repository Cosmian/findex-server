use cosmian_findex::{Address, MemoryADT, ADDRESS_LENGTH};

use super::database_traits::FindexMemoryTrait;

pub(crate) struct FindexDatabase<const WORD_LENGTH: usize> {
    pub(crate) memory: usize,
}
