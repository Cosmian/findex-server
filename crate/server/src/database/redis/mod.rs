use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
};

use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{
        redis::{build_key, FindexTable, TABLE_PREFIX_LENGTH},
        rest::UpsertData,
    },
    reexport::cosmian_findex::{
        CoreError, EncryptedValue, Token, TokenToEncryptedValueMap, TokenWithEncryptedValueList,
        Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use redis::{aio::ConnectionManager, pipe, AsyncCommands, Script};
use tracing::{instrument, trace};

use super::Database;
use crate::error::{result::FResult, server::FindexServerError};

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

#[allow(dead_code)]
pub(crate) struct Redis {
    mgr: ConnectionManager,
    upsert_script: Script,
}

impl Redis {
    pub(crate) async fn instantiate(redis_url: &str, clear_database: bool) -> FResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let mgr = ConnectionManager::new(client).await?;

        if clear_database {
            Self::clear_database(mgr.clone()).await?;
        }
        Ok(Self {
            mgr,
            upsert_script: Script::new(CONDITIONAL_UPSERT_SCRIPT),
        })
    }

    #[allow(dependency_on_unit_never_type_fallback)]
    pub(crate) async fn clear_database(mgr: ConnectionManager) -> FResult<()> {
        redis::cmd("FLUSHDB").query_async(&mut mgr.clone()).await?;
        Ok(())
    }
}

#[async_trait]
impl Database for Redis {
    // todo(manu): merge the 2 fetch
    #[instrument(ret(Display), err, skip_all)]
    async fn fetch_entries(
        &self,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>> {
        trace!("fetch_entries: number of tokens: {}", tokens.len());
        let uids = tokens.into_iter().collect::<Vec<_>>();
        trace!("fetch_entries: uids len: {}", uids.len());

        let redis_keys = uids
            .iter()
            .map(|uid| build_key(FindexTable::Entry, uid))
            .collect::<Vec<_>>();
        trace!("fetch_entries: redis_keys len: {}", redis_keys.len());

        let values: Vec<Vec<u8>> = self.mgr.clone().mget(redis_keys).await?;
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

        let result: TokenWithEncryptedValueList<ENTRY_LENGTH> = res.into();

        Ok(result)
    }

    #[instrument(ret(Display), err, skip_all)]
    async fn fetch_chains(
        &self,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>> {
        trace!("fetch_chains: number of tokens: {}", tokens.len());
        let uids = tokens.into_iter().collect::<Vec<_>>();
        trace!("fetch_chains: uids len: {}", uids.len());

        let redis_keys = uids
            .iter()
            .map(|uid| build_key(FindexTable::Chain, uid))
            .collect::<Vec<_>>();
        trace!("fetch_chains: redis_keys len: {}", redis_keys.len());

        let values: Vec<Vec<u8>> = self.mgr.clone().mget(redis_keys).await?;
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

        let result: TokenWithEncryptedValueList<LINK_LENGTH> = res.into();

        Ok(result)
    }

    #[instrument(ret(Display), err, skip_all)]
    async fn upsert_entries(
        &self,
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
            let key = build_key(FindexTable::Entry, &uid);

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

    #[instrument(ret, err, skip_all)]
    async fn insert_chains(&self, items: TokenToEncryptedValueMap<LINK_LENGTH>) -> FResult<()> {
        let mut pipe = pipe();
        for (k, v) in &*items {
            pipe.set(build_key(FindexTable::Chain, k), Vec::from(v));
        }
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip_all)]
    async fn delete(&self, findex_table: FindexTable, entry_uids: Tokens) -> FResult<()> {
        let mut pipeline = pipe();
        for uid in entry_uids {
            pipeline.del(build_key(findex_table, &uid));
        }
        pipeline
            .atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret(Display), err, skip_all)]
    #[allow(clippy::indexing_slicing)]
    async fn dump_tokens(&self) -> FResult<Tokens> {
        let keys: Vec<Vec<u8>> = self
            .mgr
            .clone()
            .keys(build_key(FindexTable::Entry, b"*"))
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
