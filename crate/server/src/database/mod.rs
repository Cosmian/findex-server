pub(crate) mod database_traits;
pub(crate) mod redis;
pub(crate) mod sqlite;

pub mod findex_database;
pub(crate) use findex_database::FindexDatabase;
pub(crate) mod error;
pub(crate) use error::DatabaseError;
pub use sqlite::{
    FINDEX_DATASETS_TABLE_NAME, FINDEX_MEMORY_TABLE_NAME, FINDEX_PERMISSIONS_TABLE_NAME,
};

pub(crate) mod test_utils;
