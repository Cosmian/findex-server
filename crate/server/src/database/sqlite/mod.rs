mod datasets;
mod findex;
mod instance;
mod memory;
mod permissions;
use cosmian_findex_structs::CUSTOM_WORD_LENGTH;

use crate::database::database_traits::DatabaseTraits;
impl DatabaseTraits for Sqlite<CUSTOM_WORD_LENGTH> {}

pub(crate) use instance::Sqlite;
pub use instance::{
    FINDEX_DATASETS_TABLE_NAME, FINDEX_MEMORY_TABLE_NAME, FINDEX_PERMISSIONS_TABLE_NAME,
};
pub(crate) use memory::SqliteMemoryError;
