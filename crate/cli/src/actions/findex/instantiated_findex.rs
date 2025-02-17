use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::error::{result::CliResult, CliError};
use cosmian_findex::{
    generic_decode, generic_encode, Address, Findex, IndexADT, MemoryADT, Value, ADDRESS_LENGTH,
};
use cosmian_findex_structs::{Keyword, Keywords, SearchResults, CUSTOM_WORD_LENGTH};
use tokio::sync::Semaphore;

use super::MAX_PERMITS;

pub struct InstantiatedFindex<
    Memory: Send
        + Sync
        + Clone
        + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; CUSTOM_WORD_LENGTH]>,
> {
    findex: Findex<CUSTOM_WORD_LENGTH, Value, String, Memory>,
}

impl<
        Memory: Send
            + Sync
            + Clone
            + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; CUSTOM_WORD_LENGTH]>
            + 'static,
    > InstantiatedFindex<Memory>
{
    pub fn new(encryption_layer: Memory) -> Self {
        Self {
            findex: Findex::new(encryption_layer, generic_encode, generic_decode),
        }
    }

    /// Search multiple keywords. Returned results are the intersection of all search results.
    ///
    /// # Errors
    /// - If search fails
    pub async fn search(&self, keyword: &[String]) -> CliResult<SearchResults> {
        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));

        let mut handles = keyword
            .iter()
            .map(|kw| kw.to_lowercase())
            .map(|k| {
                let semaphore = semaphore.clone();
                let k = Keyword::from(k.as_ref());
                let findex_instance = self.findex.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!(
                            "Acquire error while trying to ask for permit: {e:?}"
                        ))
                    })?;
                    Ok::<_, CliError>(findex_instance.search(&k).await?)
                })
            })
            .collect::<Vec<_>>();

        if let Some(initial_handle) = handles.pop() {
            let mut acc_results = initial_handle
                .await
                .map_err(|e| CliError::Default(e.to_string()))??;
            for h in handles {
                // The empty set is the fixed point of the intersection.
                if acc_results.is_empty() {
                    break;
                }
                let next_search_result =
                    h.await.map_err(|e| CliError::Default(e.to_string()))??;
                acc_results.retain(|item| next_search_result.contains(item));
            }
            Ok(SearchResults(acc_results))
        } else {
            Ok(SearchResults(HashSet::new()))
        }
    }

    /// Insert new indexes or delete indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    /// - or if delete indexes fails
    pub async fn insert_or_delete(
        &self,
        bindings: HashMap<Keyword, HashSet<Value>>,
        is_insert: bool,
    ) -> CliResult<Keywords> {
        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));
        let written_keywords = bindings.keys().cloned().collect::<Vec<_>>();

        let handles = bindings
            .into_iter()
            .map(|(kw, vs)| {
                let findex = self.findex.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!(
                            "Acquire error while trying to ask for permit: {e:?}"
                        ))
                    })?;
                    if is_insert {
                        findex.insert(kw, vs).await?;
                    } else {
                        findex.delete(kw, vs).await?;
                    }
                    Ok::<_, CliError>(())
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.await.map_err(|e| CliError::Default(e.to_string()))??;
        }

        Ok(Keywords::from(written_keywords))
    }
}
