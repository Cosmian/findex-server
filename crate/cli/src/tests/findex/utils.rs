use std::{ops::Deref, path::PathBuf};

use cosmian_findex_client::{FindexRestClient, KmsEncryptionLayer, RestClient, RestClientConfig};
use cosmian_kms_cli::{
    actions::symmetric::keys::create_key::{CreateKeyAction, SymmetricAlgorithm},
    reexport::cosmian_kms_client::{
        reexport::cosmian_http_client::HttpClientConfig, KmsClient, KmsClientConfig,
    },
};
use test_findex_server::start_default_test_findex_server;
use uuid::Uuid;

use crate::{
    actions::findex::{
        insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters, search::SearchAction,
    },
    error::result::CliResult,
    tests::search_options::SearchOptions,
};

pub(crate) const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
pub(crate) const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";

pub(crate) async fn insert_search_delete(
    findex_parameters: &FindexParameters,
    cli_conf_path: &str,
    search_options: SearchOptions,
    kms_client: KmsClient,
) -> CliResult<()> {
    let mut rest_client =
        RestClient::new(RestClientConfig::load(Some(PathBuf::from(cli_conf_path)))?)?;

    // Index the dataset
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(&search_options.dataset_path),
    }
    .insert(&mut rest_client, kms_client.clone())
    .await?;

    // Ensure searching returns the expected results
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(&mut rest_client, &kms_client)
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
    .delete(&mut rest_client, kms_client.clone())
    .await?;

    // Ensure no results are returned after deletion
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords,
    }
    .run(&mut rest_client, &kms_client)
    .await?;
    assert!(search_results.is_empty());

    Ok(())
}

pub(crate) fn instantiate_kms_client() -> CliResult<KmsClient> {
    Ok(KmsClient::new(KmsClientConfig {
        http_config: HttpClientConfig {
            server_url: format!(
                "http://{}:9998",
                std::env::var("KMS_HOSTNAME").unwrap_or_else(|_| "0.0.0.0".to_owned())
            ),
            ..HttpClientConfig::default()
        },
        ..KmsClientConfig::default()
    })?)
}

pub(crate) async fn create_encryption_layer<const WORD_LENGTH: usize>(
) -> CliResult<KmsEncryptionLayer<WORD_LENGTH, FindexRestClient<WORD_LENGTH>>> {
    let ctx = start_default_test_findex_server().await;
    let kms_client = instantiate_kms_client()?;
    let findex_parameters = new_parameters(Uuid::new_v4(), &kms_client, true).await?;

    let encryption_layer = KmsEncryptionLayer::<WORD_LENGTH, _>::new(
        kms_client.clone(),
        findex_parameters.hmac_key_id.unwrap(),
        findex_parameters.aes_xts_key_id.unwrap(),
        FindexRestClient::<WORD_LENGTH>::new(
            RestClient::new(ctx.owner_client_conf.clone())?,
            findex_parameters.index_id,
        ),
    );
    Ok(encryption_layer)
}

#[allow(clippy::as_conversions)]
pub(crate) async fn new_parameters(
    index_id: Uuid,
    kms_client: &KmsClient,
    server_side_encryption: bool,
) -> CliResult<FindexParameters> {
    async fn generate_key(
        kms_client: &KmsClient,
        bits: u32,
        algorithm: SymmetricAlgorithm,
        key_type: &str,
    ) -> CliResult<String> {
        let uid = CreateKeyAction {
            number_of_bits: Some(bits as usize),
            algorithm,
            ..CreateKeyAction::default()
        }
        .run(kms_client)
        .await?;
        println!(
            "Warning: This is the only time that this {key_type} key ID will be printed. ID: {uid}"
        );
        Ok(uid.to_string())
    }

    if server_side_encryption {
        Ok(FindexParameters {
            seed_key_id: None,
            hmac_key_id: Some(
                generate_key(kms_client, 256, SymmetricAlgorithm::Sha3, "HMAC").await?,
            ),
            aes_xts_key_id: Some(
                generate_key(kms_client, 512, SymmetricAlgorithm::Aes, "AES-XTS").await?,
            ),
            index_id,
        })
    } else {
        Ok(FindexParameters {
            seed_key_id: Some(
                generate_key(kms_client, 256, SymmetricAlgorithm::Aes, "seed").await?,
            ),
            hmac_key_id: None,
            aes_xts_key_id: None,
            index_id,
        })
    }
}
