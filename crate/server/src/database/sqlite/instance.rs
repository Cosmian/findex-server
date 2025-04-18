use crate::{database::database_traits::InstantializationTrait, error::result::FResult};
use async_sqlite::{Pool, PoolBuilder};
use async_trait::async_trait;
use cosmian_findex::{Address, SqliteMemory};
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;
use tracing::info;

pub(crate) struct Sqlite<const WORD_LENGTH: usize> {
    pub(crate) memory: SqliteMemory<Address<SERVER_ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) pool: Pool,
}

#[async_trait]
impl<const WORD_LENGTH: usize> InstantializationTrait for Sqlite<WORD_LENGTH> {
    async fn instantiate(db_url: &str, clear_database: bool) -> FResult<Self> {
        let pool = PoolBuilder::new()
            .path(db_url)
            .journal_mode(async_sqlite::JournalMode::Wal)
            .open()
            .await?;

        if clear_database {
            info!("Warning: proceeding to clear the database, this operation is irreversible.");
            pool.conn(move |conn| {
                conn.execute_batch(
                    "
                    DROP TABLE IF EXISTS findex.memory;
                    DROP TABLE IF EXISTS findex.permissions;
                    DROP TABLE IF EXISTS findex.datasets;
                    VACUUM;",
                )
            })
            .await?;
        }

        let memory =
            SqliteMemory::connect_with_pool(pool.clone(), "findex.memory".to_string()).await?;
        pool.conn(move |conn| {
            conn.execute_batch(
                "
                        PRAGMA synchronous = NORMAL;
                        CREATE TABLE IF NOT EXISTS findex.permissions (
                            user_id TEXT NOT NULL,
                            index_id BLOB NOT NULL,
                            permission INTEGER NOT NULL CHECK (permission IN (0,1,2)),
                            PRIMARY KEY (user_id, index_id)
                        );
                        CREATE TABLE findex.datasets (
                            index_id BLOB NOT NULL,  
                            uid      BLOB NOT NULL,  
                            encryptedEntry     BLOB NOT NULL, 
                            PRIMARY KEY (index_id, uid)
                        );
                        ",
            )
        })
        .await?;

        Ok(Self { memory, pool })
    }
}
