pub(crate) mod database_traits;
pub(crate) mod redis;
pub(crate) mod sqlite;

pub mod findex_database;
pub(crate) use findex_database::{DatabaseError, FindexDatabase};
