//! Findex server implements its own `SQLite` memory abstraction to avoid dependency
//! conflicts with the memory implementation provided by the upstream Findex library.
use std::{collections::HashMap, marker::PhantomData, ops::Deref};

use async_sqlite::{
    Pool,
    rusqlite::{OptionalExtension, params_from_iter},
};
use cosmian_findex::{Address, MemoryADT};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum SqliteMemoryError {
    #[error("async_sqlite error: {0}")]
    AsyncSqliteCoreError(#[from] async_sqlite::Error),
}

#[derive(Clone)]
pub(crate) struct SqliteMemory<Address, Word> {
    pool: Pool,
    table_name: String,
    _marker: PhantomData<(Address, Word)>,
}

#[allow(dead_code)]
impl<Address, Word> SqliteMemory<Address, Word> {
    /// Returns a new memory instance using a pool of connections to an `SQLite`
    /// database.
    pub(crate) const fn new_with_pool(pool: Pool, table_name: String) -> Self {
        Self {
            pool,
            table_name,
            _marker: PhantomData,
        }
    }
}

impl<const ADDRESS_LENGTH: usize, const WORD_LENGTH: usize> MemoryADT
    for SqliteMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>
{
    type Address = Address<ADDRESS_LENGTH>;
    type Error = SqliteMemoryError;
    type Word = [u8; WORD_LENGTH];

    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<Self::Word>>, Self::Error> {
        let findex_table_name = self.table_name.clone();
        self.pool
            .conn(move |conn| {
                let results = conn
                    .prepare(&format!(
                        "SELECT a, w FROM {} WHERE a IN ({})",
                        findex_table_name,
                        vec!["?"; addresses.len()].join(",")
                    ))?
                    .query_map(
                        params_from_iter(addresses.iter().map(Deref::deref)),
                        |row| {
                            let a = Address::from(row.get::<_, [u8; ADDRESS_LENGTH]>(0)?);
                            let w = row.get(1)?;
                            Ok((a, w))
                        },
                    )?
                    .collect::<Result<HashMap<_, _>, _>>()?;

                // Return order of an SQL select statement is undefined, and
                // mismatches are ignored. A post-processing is thus needed to
                // generate a returned value complying to the batch-read spec.
                Ok(addresses
                    .iter()
                    // Copying is necessary since the same word could be
                    // returned multiple times.
                    .map(|addr| results.get(addr).copied())
                    .collect())
            })
            .await
            .map_err(Self::Error::from)
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, Self::Error> {
        let findex_table_name = self.table_name.clone();
        let (ag, wg) = guard;

        self.pool
            .conn_mut(move |conn| {
                let tx = conn.transaction_with_behavior(
                    async_sqlite::rusqlite::TransactionBehavior::Immediate,
                )?;

                let current_word = tx
                    .query_row(
                        &format!("SELECT w FROM {findex_table_name} WHERE a = ?"),
                        [&*ag],
                        |row| row.get(0),
                    )
                    .optional()?;

                if current_word == wg {
                    tx.execute(
                        &format!(
                            "INSERT OR REPLACE INTO {} (a, w) VALUES {}",
                            findex_table_name,
                            vec!["(?,?)"; bindings.len()].join(",")
                        ),
                        params_from_iter(
                            bindings
                                .iter()
                                // There seems to be no way to avoid cloning here.
                                .flat_map(|(a, w)| [a.to_vec(), w.to_vec()]),
                        ),
                    )?;
                    tx.commit()?;
                }

                Ok(current_word)
            })
            .await
            .map_err(Self::Error::from)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // this is a test module, unwraps are acceptable
mod tests {

    use async_sqlite::PoolBuilder;
    use cosmian_findex::{
        gen_seed, test_guarded_write_concurrent, test_rw_same_address, test_single_write_and_read,
        test_wrong_guard,
    };

    use super::*;

    const DB_PATH: &str = "../../target/debug/sqlite-test.sqlite.db";
    const TABLE_NAME: &str = "findex_memory";

    impl<Address, Word> SqliteMemory<Address, Word> {
        async fn new_with_path(
            path: impl AsRef<std::path::Path>,
            table_name: String,
        ) -> Result<Self, SqliteMemoryError> {
            let pool = PoolBuilder::new().path(path).open().await?; // default pool size is the number of logical CPUs

            let initialization_script = format!(
                "   PRAGMA synchronous = NORMAL;
                    PRAGMA journal_mode = WAL;
                    CREATE TABLE IF NOT EXISTS {} (
                        a BLOB PRIMARY KEY,
                        w BLOB NOT NULL
                    );",
                table_name
            );

            pool.conn(move |conn| conn.execute_batch(&initialization_script))
                .await?;

            Ok(Self {
                pool,
                table_name,
                _marker: PhantomData,
            })
        }
    }

    #[tokio::test]
    async fn test_rw_seq() {
        let m = SqliteMemory::<_, [u8; 52]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
            .await
            .unwrap();
        test_single_write_and_read(&m, gen_seed()).await;
    }

    #[tokio::test]
    async fn test_guard_seq() {
        let m = SqliteMemory::<_, [u8; 999]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
            .await
            .unwrap();
        test_wrong_guard(&m, gen_seed()).await;
    }

    #[tokio::test]
    async fn test_collision_seq() {
        let m = SqliteMemory::<_, [u8; 87]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
            .await
            .unwrap();
        test_rw_same_address(&m, gen_seed()).await;
    }

    #[tokio::test]
    async fn test_rw_ccr() {
        let m = SqliteMemory::<_, [u8; 129]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
            .await
            .unwrap();
        test_guarded_write_concurrent(&m, gen_seed(), Some(100)).await;
    }
}
