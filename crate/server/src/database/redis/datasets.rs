use std::collections::HashMap;

use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, EncryptedEntries, Uuids};
use redis::pipe;
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::DatasetsTrait,
    error::{result::FResult, server::ServerError},
};

/// Generate a Redis-key for the dataset table
fn build_redis_key(index_id: &Uuid, uid: &Uuid) -> Vec<u8> {
    let mut key = Vec::with_capacity(32);
    key.extend_from_slice(index_id.as_bytes());
    key.extend_from_slice(uid.as_bytes());
    key
}

#[async_trait]
impl DatasetsTrait for Redis<CUSTOM_WORD_LENGTH> {
    //
    // Dataset management
    //
    #[instrument(ret, err, skip_all, level = "trace")]
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FResult<()> {
        entries
            .iter()
            .map(|(id, data)| (build_redis_key(index_id, id), data))
            .fold(&mut pipe(), |pipe, (key, data)| pipe.set(key, data))
            .atomic()
            .query_async(&mut self.manager.clone())
            .await
            .map_err(ServerError::from)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn dataset_delete_entries(&self, index_id: &Uuid, ids: &Uuids) -> FResult<()> {
        ids.iter()
            .map(|id| build_redis_key(index_id, id))
            .fold(&mut pipe(), |pipe, key| pipe.del(key))
            .atomic()
            .query_async(&mut self.manager.clone())
            .await
            .map_err(ServerError::from)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn dataset_get_entries(&self, index_id: &Uuid, ids: &Uuids) -> FResult<EncryptedEntries> {
        let values = ids
            .iter()
            .map(|id| build_redis_key(index_id, id))
            .fold(&mut pipe(), |pipe, key| pipe.get(key))
            .atomic()
            .query_async::<Vec<Vec<u8>>>(&mut self.manager.clone())
            .await
            .map_err(ServerError::from)?;

        trace!("dataset_get_entries: values len: {}", values.len());

        // Filter empty values out.
        let values = ids
            .iter()
            .zip(values)
            .filter_map(|(k, v)| if v.is_empty() { None } else { Some((k, v)) })
            .collect::<HashMap<_, _>>();

        Ok(values.into())
    }
}
