use async_trait::async_trait;
use cosmian_findex_structs::SERVER_ADDRESS_LENGTH;
use cosmian_sse_memories::Address;
use tokio_rusqlite::Connection;
use tracing::warn;

use crate::{
    config::DatabaseType,
    database::{
        DatabaseError, database_traits::InstantiationTrait, findex_database::DatabaseResult,
        sqlite::memory::SqliteMemory,
    },
};

pub(crate) struct Sqlite<const WORD_LENGTH: usize> {
    pub(crate) memory: SqliteMemory<Address<SERVER_ADDRESS_LENGTH>, [u8; WORD_LENGTH]>,
    pub(crate) pool: SqlitePool,
}

#[derive(Clone)]
pub(crate) struct SqlitePool {
    conn: Connection,
}

impl SqlitePool {
    pub(crate) async fn open(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, tokio_rusqlite::Error<rusqlite::Error>> {
        // NOTE: `tokio_rusqlite::Connection` is a
        // single connection serviced by a dedicated background thread.
        //
        // For SQLite this is an acceptable default because concurrency is
        // limited by database-level locking, and increasing the number of
        // connections usually does not improve throughput (and can increase
        // lock contention).
        let conn = Connection::open(path).await?;
        Ok(Self { conn })
    }

    pub(crate) async fn conn<F, R>(&self, f: F) -> Result<R, tokio_rusqlite::Error<rusqlite::Error>>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<R, rusqlite::Error> + Send + 'static,
        R: Send + 'static,
    {
        self.conn.call(f).await
    }

    pub(crate) async fn conn_mut<F, R>(
        &self,
        f: F,
    ) -> Result<R, tokio_rusqlite::Error<rusqlite::Error>>
    where
        F: FnOnce(&mut rusqlite::Connection) -> Result<R, rusqlite::Error> + Send + 'static,
        R: Send + 'static,
    {
        self.conn.call(f).await
    }
}

pub const FINDEX_MEMORY_TABLE_NAME: &str = "findex_server_memory";
pub const FINDEX_PERMISSIONS_TABLE_NAME: &str = "findex_server_permissions";
pub const FINDEX_DATASETS_TABLE_NAME: &str = "findex_server_datasets";

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
        let pool = SqlitePool::open(db_url).await?;

        if clear_database {
            warn!("clearing database, this operation is irreversible.");
            pool.conn_mut(move |conn| {
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

        let memory = SqliteMemory::new_with_pool(pool.clone(), FINDEX_MEMORY_TABLE_NAME.to_owned());
        pool.conn_mut(move |conn| {
            conn.execute_batch(&format!(
                "
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                VACUUM;
                PRAGMA auto_vacuum = 1;
                CREATE TABLE IF NOT EXISTS {FINDEX_MEMORY_TABLE_NAME} (
                    a BLOB PRIMARY KEY,
                w BLOB NOT NULL
                );
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
