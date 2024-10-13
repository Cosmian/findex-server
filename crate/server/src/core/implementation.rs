use cloudproof::reexport::crypto_core::FixedSizeCBytes;

use crate::{
    config::{DbParams, ServerParams},
    database::{Database, RedisWithFindex, SqlitePool, REDIS_WITH_FINDEX_MASTER_KEY_LENGTH},
    findex_server_bail,
    result::FResult,
    secret::Secret,
};

use super::FindexServer;

impl FindexServer {
    pub(crate) async fn instantiate(mut shared_config: ServerParams) -> FResult<Self> {
        let db: Box<dyn Database + Sync + Send> = if let Some(mut db_params) =
            shared_config.db_params.as_mut()
        {
            match &mut db_params {
                DbParams::Sqlite(db_path) => Box::new(
                    SqlitePool::instantiate(
                        &db_path.join("findex_server.db"),
                        shared_config.clear_db_on_start,
                    )
                    .await?,
                ),
                DbParams::RedisFindex(url, master_key, label) => {
                    // There is no reason to keep a copy of the key in the shared config
                    // So we are going to create a "zeroizable" copy which will be passed to Redis with Findex
                    // and zeroize the one in the shared config
                    let new_master_key =
                        Secret::<REDIS_WITH_FINDEX_MASTER_KEY_LENGTH>::from_unprotected_bytes(
                            &mut master_key.to_bytes(),
                        );
                    // `master_key` implements ZeroizeOnDrop so there is no need
                    // to manually zeroize.
                    Box::new(
                        RedisWithFindex::instantiate(url.as_str(), new_master_key, label).await?,
                    )
                }
            }
        } else {
            findex_server_bail!("Fatal: no database configuration provided. Stopping.")
        };

        Ok(Self {
            params: shared_config,
            db,
        })
    }
}
