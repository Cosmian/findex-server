use std::{fmt::Display, path::PathBuf};

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{config::params::DbParams, error::result::FResult, findex_server_error};

#[derive(ValueEnum, Clone, Deserialize, Serialize)]
pub enum DatabaseType {
    // Sqlite,
    Redis,
}

pub const DEFAULT_SQLITE_PATH: &str = "./sqlite-data";

/// Configuration for the database
#[derive(Args, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DBConfig {
    /// The database type of the Findex server
    /// - sqlite: `SQLite`. The data will be stored at the `sqlite_path`
    ///   directory
    /// - redis-findex: a Redis database with encrypted data and encrypted
    ///   indexes thanks to Findex. The Redis url must be provided, as well as
    ///   the redis-master-password and the redis-findex-label
    #[clap(long, env("FINDEX_SERVER_DATABASE_TYPE"), verbatim_doc_comment)]
    pub database_type: Option<DatabaseType>,

    /// The url of the database for findex-redis
    #[clap(
        long,
        env = "FINDEX_SERVER_DATABASE_URL",
        required_if_eq_any([("database_type", "redis-findex")]),
        default_value = "redis://localhost:6379"
    )]
    pub database_url: Option<String>,

    /// The directory path of the sqlite or sqlite-enc
    #[clap(
        long,
        env = "FINDEX_SERVER_SQLITE_PATH",
        default_value = DEFAULT_SQLITE_PATH,
        required_if_eq_any([("database_type", "sqlite")])
    )]
    pub sqlite_path: PathBuf,

    /// Clear the database on start.
    /// WARNING: This will delete ALL the data in the database
    #[clap(long, env = "FINDEX_SERVER_CLEAR_DATABASE", verbatim_doc_comment)]
    pub clear_database: bool,
}

impl Default for DBConfig {
    fn default() -> Self {
        Self {
            sqlite_path: PathBuf::from(DEFAULT_SQLITE_PATH),
            database_type: None,
            database_url: Some("redis://localhost:6379".to_owned()),
            clear_database: false,
        }
    }
}

impl Display for DBConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(database_type) = &self.database_type {
            match database_type {
                DatabaseType::Redis => write!(
                    f,
                    "redis: {}",
                    &self
                        .database_url
                        .as_ref()
                        .map_or("[INVALID LABEL]", |url| url.as_str()),
                ),
            }?;
        } else {
            write!(f, "No database configuration provided")?;
        }
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
    pub(crate) fn init(&self) -> FResult<Option<DbParams>> {
        Ok(if let Some(database_type) = &self.database_type {
            Some(match database_type {
                DatabaseType::Redis => {
                    let url = ensure_url(self.database_url.as_deref(), "FINDEX_SERVER_REDIS_URL")?;
                    DbParams::Redis(url)
                }
            })
        } else {
            return Err(findex_server_error!("No database configuration provided"));
        })
    }
}

fn ensure_url(database_url: Option<&str>, alternate_env_variable: &str) -> FResult<Url> {
    let url = database_url.map_or_else(
        || {
            std::env::var(alternate_env_variable).map_err(|_e| {
                findex_server_error!(
                    "No database URL supplied either using the 'database-url' option, or the \
                     FINDEX_SERVER_DATABASE_URL or the {alternate_env_variable} environment \
                     variables",
                )
            })
        },
        |url| Ok(url.to_owned()),
    )?;
    let url = Url::parse(&url)?;
    Ok(url)
}
