use std::collections::HashMap;

use async_trait::async_trait;
use cosmian_findex::WORD_LENGTH;
use cosmian_findex_structs::{EncryptedEntries, Uuids};
use redis::pipe;
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::DatasetsTrait,
    error::{result::FResult, server::FindexServerError},
};

/// Generate a Redis-key for the dataset table
fn build_redis_key(index_id: &Uuid, uid: &Uuid) -> Vec<u8> {
    let mut key = Vec::with_capacity(32);
    key.extend_from_slice(index_id.as_bytes());
    key.extend_from_slice(uid.as_bytes());
    key
}

#[async_trait]
impl DatasetsTrait for Redis<WORD_LENGTH> {
    //
    // Dataset management
    //
    #[instrument(ret, err, skip_all, level = "trace")]
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FResult<()> {
        let mut pipe = pipe();
        for (entry_id, data) in entries.iter() {
            let key = build_redis_key(index_id, entry_id);
            pipe.set(key, data);
        }
        pipe.atomic()
            .query_async(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> FResult<()> {
        let mut pipe = pipe();
        for entry_id in uuids.iter() {
            let key = build_redis_key(index_id, entry_id);
            pipe.del(key);
        }
        pipe.atomic()
            .query_async(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FResult<EncryptedEntries> {
        let redis_keys = uuids
            .iter()
            .map(|uid| build_redis_key(index_id, uid))
            .collect::<Vec<_>>();
        trace!("dataset_get_entries: redis_keys len: {}", redis_keys.len());

        let mut pipe = pipe();
        for key in redis_keys {
            pipe.get(key);
        }
        let values: Vec<Vec<u8>> = pipe
            .atomic()
            .query_async(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        trace!("dataset_get_entries: values len: {}", values.len());

        // Zip and filter empty values out.
        let entries = uuids
            .iter()
            .zip(values)
            .filter_map(|(k, v)| if v.is_empty() { None } else { Some((k, v)) })
            .collect::<HashMap<_, _>>();

        Ok(entries.into())
    }
}
