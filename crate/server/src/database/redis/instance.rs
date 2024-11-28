use redis::{Client, Script};
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
    pub(crate) upsert_script: Script,
}

//todo(manu):  move all test_data in root folder
impl Redis {
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;

        if clear_database {
            info!("Warning: Irreversible operation: clearing the database");
            Self::clear_database(&client).await?;
        }
        Ok(Self {
            client,
            upsert_script: Script::new(CONDITIONAL_UPSERT_SCRIPT),
        })
    }

    pub(crate) async fn clear_database(client: &Client) -> FResult<()> {
        let mut con = client.get_multiplexed_async_connection().await?;
        redis::cmd("FLUSHDB").query_async::<()>(&mut con).await?;
        Ok(())
    }
}
