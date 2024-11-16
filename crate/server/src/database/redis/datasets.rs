use std::collections::HashMap;

use async_trait::async_trait;
use cosmian_findex_structs::{EncryptedEntries, Uuids};
use redis::{pipe, AsyncCommands};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::DatasetsTrait,
    error::{result::FResult, server::FindexServerError},
};

/// Generate a key for the dataset table
fn build_dataset_key(index_id: &Uuid, uid: &Uuid) -> Vec<u8> {
    [index_id.as_bytes().as_ref(), uid.as_bytes().as_ref()].concat()
}

#[async_trait]
impl DatasetsTrait for Redis {
    //
    // Dataset management
    //
    #[instrument(ret, err, skip(self))]
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FResult<()> {
        let mut pipe = pipe();
        for (entry_id, data) in entries.iter() {
            let key = build_dataset_key(index_id, entry_id);
            pipe.set(key, data);
        }
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip(self))]
    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> FResult<()> {
        let mut pipe = pipe();
        for entry_id in uuids.iter() {
            let key = build_dataset_key(index_id, entry_id);
            pipe.del(key);
        }
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret(Display), err, skip(self))]
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FResult<EncryptedEntries> {
        let redis_keys = uuids
            .iter()
            .map(|uid| build_dataset_key(index_id, uid))
            .collect::<Vec<_>>();
        trace!("dataset_get_entries: redis_keys len: {}", redis_keys.len());

        let values: Vec<Vec<u8>> = self.mgr.clone().mget(redis_keys).await?;

        // Zip and filter empty values out.
        let entries = uuids
            .iter()
            .zip(values)
            .filter_map(|(k, v)| if v.is_empty() { None } else { Some((k, v)) })
            .collect::<HashMap<_, _>>();

        Ok(entries.into())
    }
}
