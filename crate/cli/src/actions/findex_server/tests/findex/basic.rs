use std::path::PathBuf;

use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, Value};
use cosmian_logger::log_init;
#[cfg(not(target_os = "windows"))]
use cosmian_sse_memories::test_utils::test_guarded_write_concurrent;
use cosmian_sse_memories::test_utils::{gen_seed, test_single_write_and_read, test_wrong_guard};
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use test_kms_server::start_default_test_kms_server;
use tracing::trace;
use uuid::Uuid;

use super::utils::HUGE_DATASET;
use crate::{
    actions::findex_server::{
        findex::{
            insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters,
            search::SearchAction,
        },
        tests::{
            findex::utils::{SMALL_DATASET, create_encryption_layer, insert_search_delete},
            permissions::create_index_id,
            search_options::SearchOptions,
        },
    },
    error::result::FindexCliResult,
};

pub(crate) fn findex_number_of_threads() -> Option<usize> {
    std::env::var("GITHUB_ACTIONS").map(|_| 1).ok()
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let ctx_kms = start_default_test_kms_server().await;

    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

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
        &ctx.owner_client_conf,
        search_options,
        ctx_kms.get_owner_client(),
    )
    .await?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_local_encryption() -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let ctx_kms = start_default_test_kms_server().await;

    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        false,
        findex_number_of_threads(),
    )
    .await?;

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
        &ctx.owner_client_conf,
        search_options,
        ctx_kms.get_owner_client(),
    )
    .await?;
    Ok(())
}

async fn run_huge_dataset_test(use_remote_crypto: bool) -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let ctx_kms = start_default_test_kms_server().await;

    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        use_remote_crypto,
        findex_number_of_threads(),
    )
    .await?;

    // Search 1 entry in a huge dataset
    let search_options = SearchOptions {
        dataset_path: HUGE_DATASET.into(),
        keywords: vec![
            "BDCQ.SEA1AA".to_owned(),
            "2011.06".to_owned(),
            "80078".to_owned(),
        ],
        expected_results: {
            vec![Value::from(
                "BDCQ.SEA1AA2011.0680078FNumber0Business Data Collection - BDCIndustry by \
                 employment variableFilled jobsAgriculture, Forestry and FishingActual",
            )]
            .into_iter()
            .collect()
        },
    };
    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf,
        search_options,
        ctx_kms.get_owner_client(),
    )
    .await
}

#[ignore = "takes too long for CI"]
#[tokio::test]
pub(crate) async fn test_findex_huge_dataset_remote_crypto() -> FindexCliResult<()> {
    run_huge_dataset_test(true).await
}

#[ignore = "takes too long for CI"]
#[tokio::test]
pub(crate) async fn test_findex_huge_dataset_local_crypto() -> FindexCliResult<()> {
    run_huge_dataset_test(false).await
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let ctx_kms = start_default_test_kms_server().await;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    let index_id = create_index_id(ctx.get_owner_client()).await?;
    trace!("index_id: {index_id}");

    let findex_parameters = FindexParameters::new(
        index_id,
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

    insert_search_delete(
        &findex_parameters,
        &ctx.owner_client_conf,
        search_options,
        ctx_kms.get_owner_client(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_searching_with_bad_key() -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let ctx_kms = start_default_test_kms_server().await;

    let index_id = create_index_id(ctx.get_owner_client()).await?;
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
    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

    // Index the dataset
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(&search_options.dataset_path),
    }
    .insert(ctx.get_owner_client(), ctx_kms.get_owner_client())
    .await?;

    // But change the findex keys
    // Ensures searching returns no result
    let search_results = SearchAction {
        findex_parameters: FindexParameters::new(
            Uuid::new_v4(),
            ctx_kms.get_owner_client(),
            true,
            findex_number_of_threads(),
        )
        .await?,
        keyword: search_options.keywords.clone(),
    }
    .run(ctx.get_owner_client(), ctx_kms.get_owner_client())
    .await?;
    assert!(search_results.is_empty());
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_sequential_read_write() -> FindexCliResult<()> {
    log_init(None);

    test_single_write_and_read::<CUSTOM_WORD_LENGTH, _>(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        gen_seed(),
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn test_findex_sequential_wrong_guard() -> FindexCliResult<()> {
    test_wrong_guard(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        gen_seed(),
    )
    .await;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn test_findex_concurrent_read_write() -> FindexCliResult<()> {
    test_guarded_write_concurrent::<
        CUSTOM_WORD_LENGTH,
        _,
        cosmian_findex::reexport::tokio::TokioSpawner,
    >(
        &create_encryption_layer::<CUSTOM_WORD_LENGTH>().await?,
        gen_seed(),
        Some(20),
    )
    .await;
    Ok(())
}
