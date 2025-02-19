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
    actions::findex::{
        insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters, search::SearchAction,
    },
    error::result::CliResult,
    tests::permissions::{create_index_id, list_permission, revoke_permission, set_permission},
};

use super::search_options::SearchOptions;

const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";
const TESTS_SEED: &str = "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF";

async fn insert_search_delete(
    seed: &str,
    cli_conf_path: &str,
    index_id: &Uuid,
    search_options: SearchOptions,
) -> CliResult<()> {
    let rest_client = FindexRestClient::new(&FindexClientConfig::load(Some(PathBuf::from(
        cli_conf_path,
    )))?)?;

    // Index the dataset
    InsertOrDeleteAction {
        findex_parameters: FindexParameters {
            seed: seed.to_owned(),
            index_id: *index_id,
        },
        csv: PathBuf::from(&search_options.dataset_path),
    }
    .insert(rest_client.clone())
    .await?;

    // Ensure searching returns the expected results
    let search_results = SearchAction {
        findex_parameters: FindexParameters {
            seed: seed.to_owned(),
            index_id: *index_id,
        },
        keywords: search_options.keywords.clone(),
    }
    .run(rest_client.clone())
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // Delete the dataset
    InsertOrDeleteAction {
        findex_parameters: FindexParameters {
            seed: seed.to_owned(),
            index_id: *index_id,
        },
        csv: PathBuf::from(search_options.dataset_path),
    }
    .delete(rest_client.clone())
    .await?;

    // Ensure no results are returned after deletion
    let search_results = SearchAction {
        findex_parameters: FindexParameters {
            seed: seed.to_owned(),
            index_id: *index_id,
        },
        keywords: search_options.keywords,
    }
    .run(rest_client)
    .await?;
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
    let owner_rest_client = FindexRestClient::new(&owner_conf)?;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    let index_id = create_index_id(owner_rest_client).await?;

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
pub(crate) async fn test_findex_set_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let owner_rest_client = FindexRestClient::new(&owner_conf)?;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    let index_id = create_index_id(owner_rest_client).await?;
    trace!("index_id: {index_id}");

    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let owner_rest_client = FindexRestClient::new(&owner_conf)?;

    let user_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.user_client_conf_path)))?;
    let user_rest_client = FindexRestClient::new(&user_conf)?;

    // Index the dataset as admin
    InsertOrDeleteAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(owner_rest_client.clone())
    .await?;

    set_permission(
        owner_rest_client.clone(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Read,
    )
    .await?;

    // User can read...
    let search_results = SearchAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        keywords: search_options.keywords.clone(),
    }
    .run(user_rest_client.clone())
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... but not write
    InsertOrDeleteAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(user_rest_client.clone())
    .await
    .unwrap_err();

    set_permission(
        owner_rest_client.clone(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Write,
    )
    .await?;

    let perm =
        list_permission(owner_rest_client.clone(), "user.client@acme.com".to_owned()).await?;
    debug!("User permission: {:?}", perm);

    // User can read...
    let search_results = SearchAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        keywords: search_options.keywords.clone(),
    }
    .run(user_rest_client.clone())
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... and write
    InsertOrDeleteAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(user_rest_client.clone())
    .await?;

    // Try to escalade privileges from `read` to `admin`
    set_permission(
        user_rest_client.clone(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Admin,
    )
    .await
    .unwrap_err();

    revoke_permission(
        owner_rest_client,
        "user.client@acme.com".to_owned(),
        index_id,
    )
    .await?;

    let _search_results = SearchAction {
        findex_parameters: FindexParameters {
            seed: TESTS_SEED.to_owned(),
            index_id,
        },
        keywords: search_options.keywords.clone(),
    }
    .run(user_rest_client)
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
