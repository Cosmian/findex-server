use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
};

use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{
        redis::{FindexTable, TABLE_PREFIX_LENGTH},
        rest::UpsertData,
    },
    reexport::cosmian_findex::{
        CoreError, EncryptedValue, Token, TokenToEncryptedValueMap, TokenWithEncryptedValueList,
        Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use redis::{pipe, AsyncCommands};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::instance::Redis;
use crate::{
    database::database_traits::FindexTrait,
    error::{result::FResult, server::FindexServerError},
};

/// Generate a key for the entry table or chain table
fn build_key(index_id: &Uuid, table: FindexTable, uid: &[u8]) -> Vec<u8> {
    [index_id.as_bytes().as_ref(), &[0x00, u8::from(table)], uid].concat()
}

#[async_trait]
impl FindexTrait for Redis {
    // TODO(manu): merge the 2 fetch
    #[instrument(ret(Display), err, skip_all, level = "trace")]
    async fn findex_fetch_entries(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>> {
        trace!("fetch_entries: number of tokens: {}", tokens.len());
        let uids = tokens.into_iter().collect::<Vec<_>>();
        trace!("fetch_entries: uids len: {}", uids.len());

        let redis_keys = uids
            .iter()
            .map(|uid| build_key(index_id, FindexTable::Entry, uid))
            .collect::<Vec<_>>();
        trace!("fetch_entries: redis_keys len: {}", redis_keys.len());

        // Fetch all the values in an atomic operation.
        let mut pipe = pipe();
        for key in redis_keys {
            pipe.get(key);
        }
        let values: Vec<Vec<u8>> = pipe
            .atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)?;

        trace!("findex_fetch_entries: values len: {}", values.len());

        // Zip and filter empty values out.
        let res = uids
            .into_iter()
            .zip(values)
            .filter_map(|(k, v)| {
                if v.is_empty() {
                    None
                } else {
                    Some(EncryptedValue::try_from(v.as_slice()).map(|v| (k, v)))
                }
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        trace!("fetch_entries: non empty tuples len: {}", res.len());

        Ok(res.into())
    }

    #[instrument(ret(Display), err, skip_all, level = "trace")]
    async fn findex_fetch_chains(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>> {
        trace!("fetch_chains: number of tokens: {}", tokens.len());
        let uids = tokens.into_iter().collect::<Vec<_>>();
        trace!("fetch_chains: uids len: {}", uids.len());

        let redis_keys = uids
            .iter()
            .map(|uid| build_key(index_id, FindexTable::Chain, uid))
            .collect::<Vec<_>>();
        trace!("fetch_chains: redis_keys len: {}", redis_keys.len());

        // Fetch all the values in an atomic operation.
        let mut pipe = pipe();
        for key in redis_keys {
            pipe.get(key);
        }
        let values: Vec<Vec<u8>> = pipe
            .atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)?;

        // Zip and filter empty values out.
        let res = uids
            .into_iter()
            .zip(values)
            .filter_map(|(k, v)| {
                if v.is_empty() {
                    None
                } else {
                    Some(EncryptedValue::try_from(v.as_slice()).map(|v| (k, v)))
                }
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        trace!("fetch_entries: non empty tuples len: {}", res.len());

        Ok(res.into())
    }

    #[instrument(ret(Display), err, skip_all, level = "trace")]
    async fn findex_upsert_entries(
        &self,
        index_id: &Uuid,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>> {
        trace!(
            "upsert_entries: number of upsert data: {}",
            upsert_data.len()
        );

        let mut old_values = HashMap::with_capacity(upsert_data.len());
        let mut new_values = HashMap::with_capacity(upsert_data.len());
        for (token, (old_value, new_value)) in upsert_data {
            if let Some(old_value) = old_value {
                old_values.insert(token, old_value);
            }
            new_values.insert(token, new_value);
        }

        trace!(
            "upsert_entries: number of old_values {}, number of new_values {}",
            old_values.len(),
            new_values.len()
        );

        let mut rejected = HashMap::with_capacity(new_values.len());
        for (uid, new_value) in new_values {
            let new_value = Vec::from(&new_value);
            let old_value = old_values.get(&uid).map(Vec::from).unwrap_or_default();
            let key = build_key(index_id, FindexTable::Entry, &uid);

            let indexed_value: Vec<_> = self
                .upsert_script
                .arg(key)
                .arg(old_value)
                .arg(new_value)
                .invoke_async(&mut self.mgr.clone())
                .await?;

            if !indexed_value.is_empty() {
                let encrypted_value = EncryptedValue::try_from(indexed_value.as_slice())?;
                rejected.insert(uid, encrypted_value);
            }
        }

        trace!("upsert_entries: rejected: {}", rejected.len());

        Ok(rejected.into())
    }

    #[instrument(ret, err, skip_all, level = "trace")]
    async fn findex_insert_chains(
        &self,
        index_id: &Uuid,
        items: TokenToEncryptedValueMap<LINK_LENGTH>,
    ) -> FResult<()> {
        let mut pipe = pipe();
        for (k, v) in &*items {
            pipe.set(build_key(index_id, FindexTable::Chain, k), Vec::from(v));
        }
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip_all, level = "trace")]
    async fn findex_delete(
        &self,
        index_id: &Uuid,
        findex_table: FindexTable,
        entry_uids: Tokens,
    ) -> FResult<()> {
        let mut pipeline = pipe();
        for uid in entry_uids {
            pipeline.del(build_key(index_id, findex_table, &uid));
        }
        pipeline
            .atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret(Display), err, skip_all, level = "trace")]
    #[allow(clippy::indexing_slicing)]
    async fn findex_dump_tokens(&self, index_id: &Uuid) -> FResult<Tokens> {
        let keys: Vec<Vec<u8>> = self
            .mgr
            .clone()
            .keys(build_key(index_id, FindexTable::Entry, b"*"))
            .await?;

        trace!("dumping {} keywords (ET+CT)", keys.len());

        let mut tokens_set = HashSet::new();
        for key in keys {
            if key[..TABLE_PREFIX_LENGTH] == [0x00, u8::from(FindexTable::Entry)] {
                if let Ok(token) = Token::try_from(&key[TABLE_PREFIX_LENGTH..]) {
                    tokens_set.insert(token);
                }
            }
        }
        Ok(Tokens::from(tokens_set))
    }
}
