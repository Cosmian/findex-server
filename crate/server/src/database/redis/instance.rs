use redis::{
    aio::{ConnectionManager, ConnectionManagerConfig},
    Client, Script,
};
use tracing::info;

use crate::error::result::FResult;

/// The conditional upsert script used to only update a table if the
/// indexed value matches ARGV[2]. When the value does not match, the
/// indexed value is returned.
const CONDITIONAL_UPSERT_SCRIPT: &str = r"
        local value=redis.call('GET',ARGV[1])
        if ((value==false) or (ARGV[2] == value)) then
            redis.call('SET', ARGV[1], ARGV[3])
            return
        else
            return value
        end;
    ";

pub(crate) struct Redis {
    pub(crate) client: Client,
    pub(crate) mgr: ConnectionManager,
    pub(crate) upsert_script: Script,
}

impl Redis {
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;

        let mgr = ConnectionManager::new_with_config(
            client.clone(),
            ConnectionManagerConfig::new().set_number_of_retries(18),
        )
        .await?;
        if clear_database {
            info!("Warning: Irreversible operation: clearing the database");
            Self::clear_database(mgr.clone()).await?;
        }
        Ok(Self {
            client,
            mgr,
            upsert_script: Script::new(CONDITIONAL_UPSERT_SCRIPT),
        })
    }

    pub(crate) async fn clear_database(mgr: ConnectionManager) -> FResult<()> {
        redis::cmd("FLUSHDB")
            .query_async::<()>(&mut mgr.clone())
            .await?;
        Ok(())
    }
}
