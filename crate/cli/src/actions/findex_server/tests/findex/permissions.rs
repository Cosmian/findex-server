use std::{ops::Deref, path::PathBuf};

use cosmian_findex_structs::{Permission, Value};
use cosmian_kms_cli::reexport::test_kms_server::start_default_test_kms_server;
use cosmian_logger::log_init;
use test_findex_server::start_default_test_findex_server_with_cert_auth;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{
    actions::findex_server::{
        findex::{
            insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters,
            search::SearchAction,
        },
        tests::{
            findex::{
                basic::findex_number_of_threads,
                utils::{SMALL_DATASET, insert_search_delete},
            },
            permissions::{create_index_id, list_permissions, revoke_permission, set_permission},
            search_options::SearchOptions,
        },
    },
    error::result::FindexCliResult,
};

#[tokio::test]
pub(crate) async fn test_findex_set_and_revoke_permission() -> FindexCliResult<()> {
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

    let index_id = create_index_id(ctx.get_owner_client()).await?;
    trace!("index_id: {index_id}");

    let ctx_kms = start_default_test_kms_server().await;

    let findex_parameters = FindexParameters::new(
        index_id,
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

    // Index the dataset as admin
    InsertOrDeleteAction {
        findex_parameters: findex_parameters.clone(),
        csv: PathBuf::from(SMALL_DATASET),
    }
    .insert(ctx.get_owner_client(), ctx_kms.get_owner_client())
    .await?;

    // Set read permission to the client
    set_permission(
        ctx.get_owner_client(),
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
    .run(ctx.get_user_client(), ctx_kms.get_owner_client())
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
    .insert(ctx.get_user_client(), ctx_kms.get_owner_client())
    .await
    .unwrap_err();

    // Set write permission
    set_permission(
        ctx.get_owner_client(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Write,
    )
    .await?;

    let perm = list_permissions(ctx.get_owner_client(), "user.client@acme.com".to_owned()).await?;
    debug!("User permission: {:?}", perm);

    // User can read...
    let search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(ctx.get_user_client(), ctx_kms.get_owner_client())
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
    .insert(ctx.get_user_client(), ctx_kms.get_owner_client())
    .await?;

    // Try to escalade privileges from `read` to `admin`
    set_permission(
        ctx.get_user_client(),
        "user.client@acme.com".to_owned(),
        index_id,
        Permission::Admin,
    )
    .await
    .unwrap_err();

    revoke_permission(
        ctx.get_owner_client(),
        "user.client@acme.com".to_owned(),
        index_id,
    )
    .await?;

    let _search_results = SearchAction {
        findex_parameters: findex_parameters.clone(),
        keyword: search_options.keywords.clone(),
    }
    .run(ctx.get_user_client(), ctx_kms.get_owner_client())
    .await
    .unwrap_err();

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> FindexCliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let ctx_kms = start_default_test_kms_server().await;
    let findex_parameters = FindexParameters::new(
        Uuid::new_v4(),
        ctx_kms.get_owner_client(),
        true,
        findex_number_of_threads(),
    )
    .await?;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned()],
        expected_results: {
            vec![Value::from("SouthboroughMAUnited States9686")]
                .into_iter()
                .collect()
        },
    };

    assert!(
        insert_search_delete(
            &findex_parameters,
            &ctx.user_client_conf,
            search_options,
            ctx_kms.get_owner_client(),
        )
        .await
        .is_err()
    );

    Ok(())
}
