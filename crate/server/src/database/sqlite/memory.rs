//! Findex server implements its own `SQLite` memory abstraction to avoid dependency
//! conflicts with the memory implementation provided by the upstream Findex library.
use std::{collections::HashMap, marker::PhantomData, ops::Deref};

use cosmian_sse_memories::{Address, MemoryADT};
use rusqlite::{OptionalExtension, params_from_iter};
use thiserror::Error;
use tokio_rusqlite;

use super::instance::SqlitePool;

#[derive(Error, Debug)]
pub(crate) enum SqliteMemoryError {
    #[error("sqlite error: {0}")]
    TokioRusqliteCoreError(#[from] tokio_rusqlite::Error<rusqlite::Error>),
}

#[derive(Clone)]
pub(crate) struct SqliteMemory<Address, Word> {
    pool: SqlitePool,
    table_name: String,
    _marker: PhantomData<(Address, Word)>,
}

#[allow(dead_code)]
impl<Address, Word> SqliteMemory<Address, Word> {
    /// Returns a new memory instance using a pool of connections to an `SQLite`
    /// database.
    pub(crate) const fn new_with_pool(pool: SqlitePool, table_name: String) -> Self {
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
                            let a: [u8; ADDRESS_LENGTH] = row.get(0)?;
                            let a = Address::from(a);
                            let w: [u8; WORD_LENGTH] = row.get(1)?;
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
                let tx =
                    conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

                let current_word = tx
                    .query_row(
                        &format!("SELECT w FROM {findex_table_name} WHERE a = ?"),
                        [&*ag],
                        |row| row.get(0),
                    )
                    .optional()?;

                if current_word == wg {
                    let params: Vec<Vec<u8>> = bindings
                        .iter()
                        // There seems to be no way to avoid cloning here.
                        .flat_map(|(a, w)| [a.to_vec(), w.to_vec()])
                        .collect();
                    tx.execute(
                        &format!(
                            "INSERT OR REPLACE INTO {} (a, w) VALUES {}",
                            findex_table_name,
                            vec!["(?,?)"; bindings.len()].join(",")
                        ),
                        params_from_iter(params),
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

    use cosmian_sse_memories::test_utils::{
        gen_seed, test_guarded_write_concurrent, test_rw_same_address, test_wrong_guard,
    };

    use super::*;

    const DB_PATH: &str = "sqlite-test.sqlite.db";
    const TABLE_NAME: &str = "findex_memory";

    impl<Address, Word> SqliteMemory<Address, Word> {
        async fn new_with_path(
            path: impl AsRef<std::path::Path>,
            table_name: String,
        ) -> Result<Self, SqliteMemoryError> {
            let pool = SqlitePool::open(path).await?;

            let initialization_script = format!(
                "   PRAGMA synchronous = NORMAL;
                    PRAGMA journal_mode = WAL;
                    CREATE TABLE IF NOT EXISTS {table_name} (
                        a BLOB PRIMARY KEY,
                        w BLOB NOT NULL
                    );"
            );

            pool.conn_mut(move |conn| conn.execute_batch(&initialization_script))
                .await?;

            Ok(Self {
                pool,
                table_name,
                _marker: PhantomData,
            })
        }
    }

    // The test below is disabled because it is flaky on CI with error:
    // thread 'database::sqlite::memory::tests::test_sequential_read_write' panicked at crate/server/src/database/sqlite/memory.rs:174:14:
    // called `Result::unwrap()` on an `Err` value: AsyncSqliteCoreError(Rusqlite(SqliteFailure(Error { code: DatabaseBusy, extended_code: 5 }, Some("database is locked"))))
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // #[tokio::test]
    // async fn test_sequential_read_write() {
    //     let m = SqliteMemory::<_, [u8; 52]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
    //         .await
    //         .unwrap();
    //     test_single_write_and_read(&m, gen_seed()).await;
    // }

    #[tokio::test]
    async fn test_sequential_wrong_guard() {
        let m = SqliteMemory::<_, [u8; 999]>::new_with_path(DB_PATH, TABLE_NAME.to_owned())
            .await
            .unwrap();
        test_wrong_guard(&m, gen_seed()).await;
    }

    #[tokio::test]
    async fn test_sequential_same_address() {
        let m = SqliteMemory::<_, [u8; 87]>::new_with_path(
            format!("test_sequential_same_address{DB_PATH}"),
            TABLE_NAME.to_owned(),
        )
        .await
        .unwrap();
        test_rw_same_address(&m, gen_seed()).await;
    }

    #[tokio::test]
    // This test is ran on a different table to avoid sqlite locking issues on some systems.
    // It can be flaky on some systems, do not hesitate to re-run it if it makes an automated workflow fail.
    async fn test_concurrent_read_write() {
        const WORD_LENGTH: usize = 129;
        let m = SqliteMemory::<_, [u8; WORD_LENGTH]>::new_with_path(
            format!("test_concurrent_read_write{DB_PATH}"),
            TABLE_NAME.to_owned(),
        )
        .await
        .unwrap();
        test_guarded_write_concurrent::<WORD_LENGTH, _>(&m, gen_seed(), Some(100)).await;
    }
}
