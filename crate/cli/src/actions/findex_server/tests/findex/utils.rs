use std::{ops::Deref, path::PathBuf};

use cosmian_findex_client::{FindexRestClient, KmsEncryptionLayer, RestClient, RestClientConfig};
use cosmian_kms_cli::reexport::{
    cosmian_kms_client::KmsClient, test_kms_server::start_default_test_kms_server,
};
use test_findex_server::start_default_test_findex_server;
use uuid::Uuid;

use super::basic::findex_number_of_threads;
use crate::{
    actions::findex_server::{
        findex::{
            insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters,
            search::SearchAction,
        },
        tests::search_options::SearchOptions,
    },
    error::result::FindexCliResult,
};

pub(crate) const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
pub(crate) const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";

pub(crate) async fn insert_search_delete(
    findex_parameters: &FindexParameters,
    config: &RestClientConfig,
    search_options: SearchOptions,
    kms_client: KmsClient,
) -> FindexCliResult<()> {
    let rest_client = RestClient::new(config.clone())?;

    // Index the dataset
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(&search_options.dataset_path),
    }
    .insert(rest_client.clone(), kms_client.clone())
    .await?;

    // Ensure searching returns the expected results
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(rest_client.clone(), kms_client.clone())
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // Delete the dataset
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(search_options.dataset_path),
    }
    .delete(rest_client.clone(), kms_client.clone())
    .await?;

    // Ensure no results are returned after deletion
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords,
    }
    .run(rest_client.clone(), kms_client)
    .await?;
    assert!(search_results.is_empty());

    Ok(())
}

pub(crate) async fn create_encryption_layer<const WORD_LENGTH: usize>()
-> FindexCliResult<KmsEncryptionLayer<WORD_LENGTH, FindexRestClient<WORD_LENGTH>>> {
    let (ctx_findex, ctx_kms) = tokio::join!(
        start_default_test_findex_server(),
        start_default_test_kms_server()
    );

    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

    let encryption_layer = KmsEncryptionLayer::<WORD_LENGTH, _>::new(
        ctx_kms.get_owner_client(),
        findex_parameters.hmac_key_id.unwrap(),
        findex_parameters.aes_xts_key_id.unwrap(),
        FindexRestClient::<WORD_LENGTH>::new(
            ctx_findex.get_owner_client(),
            findex_parameters.index_id,
        ),
    );
    Ok(encryption_layer)
}
