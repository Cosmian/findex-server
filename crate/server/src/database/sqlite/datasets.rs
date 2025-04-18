use std::collections::HashMap;

use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, EncryptedEntries, Uuids};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Sqlite;
use crate::{
    database::database_traits::DatasetsTrait,
    error::{result::FResult, server::ServerError},
};

#[async_trait]
impl DatasetsTrait for Sqlite<CUSTOM_WORD_LENGTH> {
    //
    // Dataset management
    //
    #[instrument(ret, err, skip_all, level = "trace")]
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FResult<()> {
        todo!("dataset_add_entries: entries: {:?}", entries);
        // entries
        //     .iter()
        //     .map(|(id, data)| (build_redis_key(index_id, id), data))
        //     .fold(&mut pipe(), |pipe, (key, data)| pipe.set(key, data))
        //     .atomic()
        //     .query_async(&mut self.manager.clone())
        //     .await
        //     .map_err(ServerError::from)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn dataset_delete_entries(&self, index_id: &Uuid, ids: &Uuids) -> FResult<()> {
        todo!("dataset_delete_entries: ids: {:?}", ids);
        // ids.iter()
        //     .map(|id| build_redis_key(index_id, id))
        //     .fold(&mut pipe(), |pipe, key| pipe.del(key))
        //     .atomic()
        //     .query_async(&mut self.manager.clone())
        //     .await
        //     .map_err(ServerError::from)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn dataset_get_entries(&self, index_id: &Uuid, ids: &Uuids) -> FResult<EncryptedEntries> {
        todo!("dataset_get_entries: ids: {:?}", ids);
        // let values = ids
        //     .iter()
        //     .map(|id| build_redis_key(index_id, id))
        //     .fold(&mut pipe(), |pipe, key| pipe.get(key))
        //     .atomic()
        //     .query_async::<Vec<Vec<u8>>>(&mut self.manager.clone())
        //     .await
        //     .map_err(ServerError::from)?;

        // trace!("dataset_get_entries: values len: {}", values.len());

        // // Filter empty values out.
        // let values = ids
        //     .iter()
        //     .zip(values)
        //     .filter_map(|(k, v)| if v.is_empty() { None } else { Some((k, v)) })
        //     .collect::<HashMap<_, _>>();

        // Ok(values.into())
    }
}
