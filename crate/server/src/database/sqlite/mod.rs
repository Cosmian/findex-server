use std::{path::Path, time::Duration};

use async_trait::async_trait;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, Executor, Pool, Sqlite,
};

use crate::{
    database::{Database, SQLITE_QUERIES},
    findex_server_error,
    result::FResult,
};

#[macro_export]
macro_rules! get_sqlite_query {
    ($name:literal) => {
        SQLITE_QUERIES
            .get($name)
            .ok_or_else(|| findex_server_error!("{} SQL query can't be found", $name))?
    };
    ($name:expr) => {
        SQLITE_QUERIES
            .get($name)
            .ok_or_else(|| findex_server_error!("{} SQL query can't be found", $name))?
    };
}

#[allow(dead_code)]
pub(crate) struct SqlitePool {
    pool: Pool<Sqlite>,
}

impl SqlitePool {
    /// Instantiate a new `SQLite` database
    /// and create the appropriate table(s) if need be
    pub(crate) async fn instantiate(path: &Path, clear_database: bool) -> FResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            // Sets a timeout value to wait when the database is locked, before returning a busy timeout error.
            .busy_timeout(Duration::from_secs(120))
            .create_if_missing(true)
            // disable logging of each query
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .max_connections(u32::try_from(num_cpus::get())?)
            .connect_with(options)
            .await?;

        sqlx::query(get_sqlite_query!("create-table-context"))
            .execute(&pool)
            .await?;

        sqlx::query(get_sqlite_query!("create-table-objects"))
            .execute(&pool)
            .await?;

        sqlx::query(get_sqlite_query!("create-table-read_access"))
            .execute(&pool)
            .await?;

        sqlx::query(get_sqlite_query!("create-table-tags"))
            .execute(&pool)
            .await?;

        if clear_database {
            clear_database_(&pool).await?;
        }

        let sqlite_pool = Self { pool };
        Ok(sqlite_pool)
    }
}

#[async_trait(?Send)]
impl Database for SqlitePool {
    async fn create(&self) -> FResult<()> {
        Ok(())
    }
}

pub(crate) async fn clear_database_<'e, E>(executor: E) -> FResult<()>
where
    E: Executor<'e, Database = Sqlite> + Copy,
{
    // Erase `context` table
    sqlx::query(get_sqlite_query!("clean-table-context"))
        .execute(executor)
        .await?;
    // Erase `objects` table
    sqlx::query(get_sqlite_query!("clean-table-objects"))
        .execute(executor)
        .await?;
    // Erase `read_access` table
    sqlx::query(get_sqlite_query!("clean-table-read_access"))
        .execute(executor)
        .await?;
    // Erase `tags` table
    sqlx::query(get_sqlite_query!("clean-table-tags"))
        .execute(executor)
        .await?;
    Ok(())
}
