use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    actions::findex::retrieve_key_from_kms,
    error::{result::CliResult, CliError},
};
use cosmian_findex::{
    generic_decode, generic_encode, Findex, IndexADT, MemoryEncryptionLayer, Value,
};
use cosmian_findex_client::{FindexRestClient, KmsEncryptionLayer, RestClient};
use cosmian_findex_structs::{Keyword, Keywords, SearchResults};
use cosmian_kms_cli::reexport::cosmian_kms_client::KmsClient;
use tokio::sync::Semaphore;
use tracing::trace;

use super::{parameters::FindexParameters, MAX_PERMITS};

#[derive(Clone)]
pub enum FindexInstance<const WORD_LENGTH: usize> {
    ClientSideEncryption(
        Findex<
            WORD_LENGTH,
            Value,
            String,
            MemoryEncryptionLayer<WORD_LENGTH, FindexRestClient<WORD_LENGTH>>,
        >,
    ),
    KmsEncryption(
        Findex<
            WORD_LENGTH,
            Value,
            String,
            KmsEncryptionLayer<WORD_LENGTH, FindexRestClient<WORD_LENGTH>>,
        >,
    ),
}

impl<const WORD_LENGTH: usize> FindexInstance<WORD_LENGTH> {
    /// Instantiates a new Findex instance.
    /// If a seed key is provided, the client side encryption is used.
    /// Otherwise, the KMS server-side encryption is used.
    ///
    /// # Errors
    /// - If the seed key cannot be retrieved from the KMS
    /// - If the HMAC key ID or the AES XTS key ID cannot be retrieved from the KMS
    pub async fn instantiate_findex(
        rest_client: &RestClient,
        kms_client: KmsClient,
        findex_parameters: &FindexParameters,
    ) -> CliResult<Self> {
        let memory = FindexRestClient::new(rest_client.clone(), findex_parameters.index_id);

        Ok(if let Some(seed_key_id) = &findex_parameters.seed_key_id {
            trace!("Using client side encryption");
            let seed = retrieve_key_from_kms(seed_key_id, kms_client).await?;
            let encryption_layer = MemoryEncryptionLayer::<WORD_LENGTH, _>::new(&seed, memory);
            Self::ClientSideEncryption(Findex::new(
                encryption_layer,
                generic_encode,
                generic_decode,
            ))
        } else {
            trace!("Using KMS server side encryption");
            let hmac_key_id = findex_parameters.get_hmac_key_id(&kms_client).await?;
            let aes_xts_key_id = findex_parameters.get_aes_xts_key_id(&kms_client).await?;
            let encryption_layer = KmsEncryptionLayer::<WORD_LENGTH, _>::new(
                kms_client,
                hmac_key_id,
                aes_xts_key_id,
                memory,
            );
            Self::KmsEncryption(Findex::new(
                encryption_layer,
                generic_encode,
                generic_decode,
            ))
        })
    }

    /// Search multiple keywords. Returned results are the intersection of all search results (logical AND).
    ///
    /// # Errors
    /// - If any of the concurrent search operations fail:
    /// - If the semaphore acquisition fails due to system resource exhaustion
    pub async fn search(&self, keywords: &[String]) -> CliResult<SearchResults> {
        let lowercase_keywords = keywords
            .iter()
            .map(|kw| kw.to_lowercase())
            .collect::<Vec<_>>();

        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));

        let mut handles = lowercase_keywords
            .iter()
            .map(|kw| {
                let semaphore = semaphore.clone();
                let keyword = Keyword::from(kw.as_ref());
                let findex_instance = self.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!(
                            "Acquire error while trying to ask for permit: {e:?}"
                        ))
                    })?;
                    Ok::<_, CliError>(match findex_instance {
                        Self::ClientSideEncryption(findex) => findex.search(&keyword).await?,
                        Self::KmsEncryption(findex) => findex.search(&keyword).await?,
                    })
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
                let findex = self.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!(
                            "Acquire error while trying to ask for permit: {e:?}"
                        ))
                    })?;
                    match findex {
                        Self::ClientSideEncryption(findex) => {
                            if is_insert {
                                findex.insert(kw, vs).await?;
                            } else {
                                findex.delete(kw, vs).await?;
                            }
                        }
                        Self::KmsEncryption(findex) => {
                            if is_insert {
                                findex.insert(kw, vs).await?;
                            } else {
                                findex.delete(kw, vs).await?;
                            }
                        }
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
