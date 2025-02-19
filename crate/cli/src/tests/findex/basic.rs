use std::path::PathBuf;

use cosmian_findex::{
    test_utils::{test_guarded_write_concurrent, test_single_write_and_read, test_wrong_guard},
    Value,
};
use cosmian_findex_client::RestClient;
use cosmian_findex_structs::CUSTOM_WORD_LENGTH;
use cosmian_logger::log_init;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use tracing::trace;
use uuid::Uuid;

use crate::{
    actions::findex::{
        insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters, search::SearchAction,
    },
    error::result::CliResult,
    tests::{
        findex::utils::{
            create_encryption_layer, insert_search_delete, instantiate_kms_client, HUGE_DATASET,
            SMALL_DATASET,
        },
        permissions::create_index_id,
        search_options::SearchOptions,
    },
};

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let kms_client = instantiate_kms_client()?;
    let findex_parameters =
        FindexParameters::new_with_encryption_keys(Uuid::new_v4(), &kms_client).await?;

    // Search 2 entries in a small dataset. Expect 2 results.
    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };
    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf_path,
        search_options,
        kms_client,
    )
    .await?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_local_encryption() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let kms_client = instantiate_kms_client()?;
    let findex_parameters =
        FindexParameters::new_for_client_side_encryption(Uuid::new_v4(), &kms_client).await?;

    // Search 2 entries in a small dataset. Expect 2 results.
    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };
    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf_path,
        search_options,
        kms_client,
    )
    .await?;
    Ok(())
}

#[ignore]
#[tokio::test]
pub(crate) async fn test_findex_no_auth_huge_dataset() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let kms_client = instantiate_kms_client()?;
    let findex_parameters =
    // FindexParameters::new_for_client_side_encryption(Uuid::new_v4(), &kms_client).await?;
    FindexParameters::new_with_encryption_keys(Uuid::new_v4(), &kms_client).await?;

    // Search 1 entry in a huge dataset
    let search_options = SearchOptions {
        dataset_path: HUGE_DATASET.into(),
        keywords: vec![
            "BDCQ.SEA1AA".to_owned(),
            "2011.06".to_owned(),
            "80078".to_owned(),
        ],
        expected_results: {
            vec![
                Value::from("BDCQ.SEA1AA2011.0680078FNumber0Business Data Collection - BDCIndustry by employment variableFilled jobsAgriculture, Forestry and FishingActual"),
            ]
            .into_iter()
            .collect()
        },
    };
    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf_path,
        search_options,
        kms_client,
    )
    .await?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_rest_client = RestClient::new(ctx.owner_client_conf.clone())?;
    let kms_client = instantiate_kms_client()?;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    let index_id = create_index_id(&owner_rest_client).await?;
    trace!("index_id: {index_id}");

    let findex_parameters =
        FindexParameters::new_with_encryption_keys(index_id, &kms_client).await?;

    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf_path,
        search_options,
        kms_client,
    )
    .await?;

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth_searching_with_bad_key() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    let mut rest_client = RestClient::new(ctx.owner_client_conf.clone())?;
    let kms_client = instantiate_kms_client()?;

    let index_id = create_index_id(&rest_client).await?;
    trace!("index_id: {index_id}");

    // Search 2 entries in a small dataset. Expect 2 results.
    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };
    let findex_parameters =
        FindexParameters::new_for_client_side_encryption(index_id, &kms_client).await?;

    // Index the dataset
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(&search_options.dataset_path),
    }
    .insert(&mut rest_client, kms_client.clone())
    .await?;

    // But change the findex keys
    // Ensures searching returns no result
    let search_results = SearchAction {
        findex_parameters: FindexParameters::new_with_encryption_keys(index_id, &kms_client)
            .await?,
        keyword: search_options.keywords.clone(),
    }
    .run(&mut rest_client, &kms_client)
    .await?;
    assert!(search_results.is_empty());
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_sequential_read_write() -> CliResult<()> {
    log_init(None);

    test_single_write_and_read::<CUSTOM_WORD_LENGTH, _>(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        rand::random(),
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn test_findex_sequential_wrong_guard() -> CliResult<()> {
    test_wrong_guard(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        rand::random(),
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn test_findex_concurrent_read_write() -> CliResult<()> {
    test_guarded_write_concurrent(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        rand::random(),
    )
    .await;
    Ok(())
}
