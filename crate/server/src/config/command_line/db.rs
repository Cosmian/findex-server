use std::{fmt::Display, path::PathBuf};

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use url::Url;
#[cfg(test)]
use variant_count::VariantCount;

pub(crate) const DEFAULT_SQLITE_PATH: &str = "../../target/sqlite-data.db";

#[cfg_attr(test, derive(VariantCount))] // Used only in some tests to make sure they stay up to date after a new database type is added
#[derive(ValueEnum, Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum DatabaseType {
    Redis,
    Sqlite,
}

/// Configuration for the database
#[derive(Args, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct DBConfig {
    /// The database type of the Findex server
    /// - redis: Redis database. The Redis url must be provided
    /// - sqlite: `SQLite` database. The `SQLite` file path must be provided
    #[clap(
        long,
        env = "FINDEX_SERVER_DATABASE_TYPE",
        default_value = "redis",
        value_enum,
        verbatim_doc_comment
    )]
    pub database_type: DatabaseType,

    /// The url of the database
    /// - redis: The Redis url. Default is `redis://localhost:6379`
    /// - sqlite: The `SQLite` file path, for example `./sqlite-data.db`
    #[clap(
        long,
        env = "FINDEX_SERVER_DATABASE_URL",
        required_if_eq_any([("database_type", "redis"),("database_type", "sqlite")]),
        default_value = "redis://localhost:6379",
        verbatim_doc_comment
    )]
    pub database_url: String,

    /// Clear the database on start.
    /// WARNING: This will delete ALL the data in the database
    #[clap(
        long,
        env = "FINDEX_SERVER_CLEAR_DATABASE",
        verbatim_doc_comment,
        default_value = "false"
    )]
    pub clear_database: bool,
}

impl Default for DBConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::Redis,
            database_url: "redis://localhost:6379".to_owned(),
            clear_database: false,
        }
    }
}

impl Display for DBConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.database_type {
            DatabaseType::Redis => write!(f, "redis: {}", self.database_url),
            DatabaseType::Sqlite => write!(f, "sqlite: {}", self.database_url),
        }?;
        write!(f, ", clear_database?: {}", self.clear_database)
    }
}

impl std::fmt::Debug for DBConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self))
    }
}

impl DBConfig {
    /// Initialize the DB parameters based on the command-line parameters
    ///
    /// # Parameters
    /// - `workspace`: The workspace configuration used to determine the public
    ///   and shared paths
    ///
    /// # Returns
    /// - The DB parameters
    pub(crate) fn init(&self) -> FResult<DbParams> {
        match self.database_type {
            DatabaseType::Redis => {
                let url = ensure_url(self.database_url.as_str(), "FINDEX_SERVER_REDIS_URL")?;
                Ok(DbParams::Redis(url))
            }
            DatabaseType::Sqlite => {
                let path =
                    ensure_sqlite_db(self.database_url.as_str(), "FINDEX_SERVER_SQLITE_URL")?;
                Ok(DbParams::Sqlite(path))
            }
        }
    }
}

fn retrieve_database_location(database_url: &str, alternate_env_variable: &str) -> FResult<String> {
    Ok(if database_url.is_empty() {
        std::env::var(alternate_env_variable).map_err(|_e| {
            findex_server_error!(
                "No database URL supplied either using the 'database-url' option, or the \
                 FINDEX_SERVER_DATABASE_URL or the {alternate_env_variable} environment variables.",
            )
        })?
    } else {
        database_url.to_owned()
    })
}

fn ensure_url(database_url: &str, alternate_env_variable: &str) -> FResult<Url> {
    Ok(Url::parse(&retrieve_database_location(
        database_url,
        alternate_env_variable,
    )?)?)
}

// Open and immediatly close a connection from the provided path to check if it is valid
// This creates the database if it does not exist, and tries to open it if it does
fn ensure_sqlite_db(database_url: &str, alternate_env_variable: &str) -> FResult<PathBuf> {
    let path = &retrieve_database_location(database_url, alternate_env_variable)?;
    drop(
        async_sqlite::rusqlite::Connection::open(path).map_err(|e| {
            findex_server_error!("Failed to open SQLite database at {}: {}", path, e)
        })?,
    );
    Ok(PathBuf::from(path))
}
