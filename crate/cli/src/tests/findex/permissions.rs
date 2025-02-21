use crate::{
    actions::findex::{
        insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters, search::SearchAction,
    },
    error::result::CliResult,
    tests::{
        findex::utils::{insert_search_delete, instantiate_kms_client, SMALL_DATASET},
        permissions::{create_index_id, list_permissions, revoke_permission, set_permission},
        search_options::SearchOptions,
    },
};
use cosmian_findex::Value;
use cosmian_findex_client::{RestClient, RestClientConfig};
use cosmian_findex_structs::Permission;
use cosmian_logger::log_init;
use std::{ops::Deref, path::PathBuf};
use test_findex_server::start_default_test_findex_server_with_cert_auth;
use tracing::{debug, trace};
use uuid::Uuid;

#[tokio::test]
pub(crate) async fn test_findex_set_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_rest_client = RestClient::new(&ctx.owner_client_conf.clone())?;

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

    let owner_conf = RestClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let mut owner_rest_client = RestClient::new(&owner_conf)?;

    let user_conf = RestClientConfig::load(Some(PathBuf::from(&ctx.user_client_conf_path)))?;
    let mut user_rest_client = RestClient::new(&user_conf)?;

    let kms_client = instantiate_kms_client()?;
    let findex_parameters = FindexParameters::new(index_id, &kms_client, true).await?;

    // Index the dataset as admin
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(&mut owner_rest_client, kms_client.clone())
    .await?;

    // Set read permission to the client
    set_permission(
        owner_rest_client.clone(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Read,
    )
    .await?;

    // User can read...
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(&mut user_rest_client, &kms_client)
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... but not write
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(&mut user_rest_client, kms_client.clone())
    .await
    .unwrap_err();

    // Set write permission
    set_permission(
        owner_rest_client.clone(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Write,
    )
    .await?;

    let perm =
        list_permissions(owner_rest_client.clone(), "user.client@acme.com".to_owned()).await?;
    debug!("User permission: {:?}", perm);

    // User can read...
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(&mut user_rest_client, &kms_client)
    .await?;
    assert_eq!(
        search_options.expected_results,
        search_results.deref().clone()
    );

    // ... and write
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(&mut user_rest_client, kms_client.clone())
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
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(&mut user_rest_client, &kms_client)
    .await
    .unwrap_err();

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let kms_client = instantiate_kms_client()?;
    let findex_parameters = FindexParameters::new(Uuid::new_v4(), &kms_client, true).await?;

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
        &findex_parameters,
        &ctx.user_client_conf_path,
        search_options,
        kms_client
    )
    .await
    .is_err());

    Ok(())
}
