use async_sqlite::{Pool, PoolBuilder};
use async_trait::async_trait;
use cosmian_findex::{Address, SqliteMemory};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;
use tracing::warn;

use crate::{
    config::DatabaseType,
    database::{
        DatabaseError, database_traits::InstantiationTrait, findex_database::DatabaseResult,
    },
};

pub(crate) struct Sqlite<const WORD_LENGTH: usize> {
    pub(crate) memory: SqliteMemory<Address<SERVER_ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) pool: Pool,
}

pub use cosmian_findex::FINDEX_TABLE_NAME as FINDEX_MEMORY_TABLE_NAME;
pub const FINDEX_PERMISSIONS_TABLE_NAME: &str = "findex_permissions";
pub const FINDEX_DATASETS_TABLE_NAME: &str = "findex_datasets";

#[async_trait]
#[allow(clippy::expect_used)]
impl<const WORD_LENGTH: usize> InstantiationTrait for Sqlite<WORD_LENGTH> {
    async fn instantiate(
        db_type: DatabaseType,
        db_url: &str,
        clear_database: bool,
    ) -> DatabaseResult<Self> {
        if db_type != DatabaseType::Sqlite {
            return Err(DatabaseError::InvalidDatabaseType(
                "Sqlite".to_owned(),
                format!("{db_type:?}"),
            ));
        }
        let pool = PoolBuilder::new().path(db_url).open().await?;

        if clear_database {
            warn!("clearing database, this operation is irreversible.");
            pool.conn(move |conn| {
                conn.execute_batch(&format!(
                    "
                    DROP TABLE IF EXISTS {FINDEX_MEMORY_TABLE_NAME};
                    DROP TABLE IF EXISTS {FINDEX_PERMISSIONS_TABLE_NAME};
                    DROP TABLE IF EXISTS {FINDEX_DATASETS_TABLE_NAME};
                    ",
                ))
            })
            .await?;
        }

        let memory =
            SqliteMemory::connect_with_pool(pool.clone(), FINDEX_MEMORY_TABLE_NAME.to_owned())
                .await?;
        pool.conn(move |conn| {
            conn.execute_batch(&format!(
                "
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                VACUUM;
                PRAGMA auto_vacuum = 1;
                CREATE TABLE IF NOT EXISTS {FINDEX_PERMISSIONS_TABLE_NAME} (
                    user_id TEXT NOT NULL,
                    index_id BLOB NOT NULL,
                    permission INTEGER NOT NULL CHECK (permission IN (0,1,2)),
                    PRIMARY KEY (user_id, index_id)
                );
                CREATE TABLE IF NOT EXISTS  {FINDEX_DATASETS_TABLE_NAME} (
                    index_id BLOB NOT NULL,  
                    user_id      BLOB NOT NULL,  
                    encrypted_entry     BLOB NOT NULL, 
                    PRIMARY KEY (index_id, user_id)
                );
                ",
            ))
        })
        .await?;

        Ok(Self { memory, pool })
    }
}
