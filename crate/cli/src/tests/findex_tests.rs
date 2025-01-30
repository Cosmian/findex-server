use std::{ops::Deref, path::PathBuf};

use cosmian_findex::Value;
use cosmian_findex_client::{FindexClientConfig, FindexRestClient};
use cosmian_findex_structs::Permission;
use cosmian_logger::log_init;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{
    error::result::CliResult,
    tests::{
        permissions::{create_index_id, grant_permission, list_permission, revoke_permission},
        utils::{delete, insert, search},
    },
};

use super::utils::SearchOptions;

const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";
const TESTS_SEED: &str = "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF";

async fn insert_search_delete(
    seed: &str,
    cli_conf_path: &str,
    index_id: &Uuid,
    search_options: SearchOptions,
) -> CliResult<()> {
    let test_conf = FindexClientConfig::load(Some(PathBuf::from(cli_conf_path)))?;
    let mut rest_client: FindexRestClient = FindexRestClient::new(test_conf)?;

    insert(
        seed.clone(),
        *index_id,
        &search_options.dataset_path,
        &mut rest_client,
    )
    .await?;

    // make sure searching returns the expected results
    let search_results = search(seed, *index_id, search_options.clone(), &mut rest_client).await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    debug!("Search results: {}", search_results);

    delete(
        seed.clone(),
        *index_id,
        &search_options.dataset_path,
        &mut rest_client,
    )
    .await?;

    // make sure no results are returned after deletion
    let search_results = search(seed, *index_id, search_options, &mut rest_client).await?;
    assert!(search_results.is_empty());

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

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
        TESTS_SEED,
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        search_options,
    )
    .await?;
    Ok(())
}

#[ignore]
#[tokio::test]
pub(crate) async fn test_findex_no_auth_huge_dataset() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

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
        TESTS_SEED,
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        search_options,
    )
    .await?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let owner_rest_client = FindexRestClient::new(owner_conf)?;

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

    insert_search_delete(
        TESTS_SEED,
        &ctx.owner_client_conf_path,
        &index_id,
        search_options,
    )
    .await?;

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_grant_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let owner_rest_client = FindexRestClient::new(owner_conf)?;

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

    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let mut owner_rest_client = FindexRestClient::new(owner_conf)?;

    let user_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.user_client_conf_path)))?;
    let mut user_rest_client = FindexRestClient::new(user_conf)?;

    insert(TESTS_SEED, &index_id, SMALL_DATASET, &mut owner_rest_client).await?;

    // Grant read permission to the client
    grant_permission(
        &owner_rest_client,
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Read,
    )
    .await?;

    // User can read...
    let search_results = search(
        TESTS_SEED,
        &index_id,
        &search_options,
        &mut user_rest_client,
    )
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... but not write
    assert!(
        insert(TESTS_SEED, &index_id, SMALL_DATASET, &mut user_rest_client)
            .await
            .is_err()
    );

    // Grant write permission
    grant_permission(
        &owner_rest_client,
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Write,
    )
    .await?;

    let perm = list_permission(&owner_rest_client, "user.client@acme.com".to_owned()).await?;
    debug!("User permission: {:?}", perm);

    // User can read...
    let search_results = search(
        TESTS_SEED,
        &index_id,
        &search_options,
        &mut user_rest_client,
    )
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... and write
    insert(TESTS_SEED, &index_id, SMALL_DATASET, &mut user_rest_client).await?;

    // Try to escalade privileges from `read` to `admin`
    grant_permission(
        &user_rest_client,
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Admin,
    )
    .await
    .unwrap_err();

    revoke_permission(
        &owner_rest_client,
        "user.client@acme.com".to_owned(),
        index_id,
    )
    .await?;

    let _search_results = search(
        TESTS_SEED,
        &index_id,
        &search_options,
        &mut user_rest_client,
    )
    .await
    .unwrap_err();

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    assert!(insert_search_delete(
        TESTS_SEED,
        &ctx.user_client_conf_path,
        &Uuid::new_v4(),
        search_options
    )
    .await
    .is_err());

    Ok(())
}
