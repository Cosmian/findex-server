use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

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
        utils::{add, delete, search},
    },
};

use super::utils::SearchOptions;

const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";

/// Helper function for `parse_locations`, docs below
fn parse_value_string(input: &str) -> Vec<Vec<u8>> {
    if input == "{}" {
        return vec![];
    }
    let values: Vec<Vec<u8>> = input
        .split("Value([")
        .skip(1)
        .map(|s| {
            s.split("])")
                .next()
                .unwrap_or("")
                .split(", ")
                .filter_map(|num| num.parse().ok())
                .collect()
        })
        .collect();

    values
}

/// This is a helper function
/// Findex v7 returns a string with a format similar to :
/// "Location1: Value([1, 2, 3])\nLocation2: Value([4, 5, 6])\n"
/// This function parses this string into a hashmap, where the key is the location and the value is a set of values.
/// It returns a tuple with the adequate string representation of the hashmap and the hashmap itself.
#[allow(clippy::cognitive_complexity)] // function is already simplified and usage is obvious, no need to simplify further
fn parse_locations(input: &str, verbose: bool) -> (String, HashMap<String, HashSet<String>>) {
    let mut result_set: HashMap<String, HashSet<String>> = HashMap::new();
    for part in input.split('\n') {
        if let Some((location, values)) = part.split_once(": ") {
            let location = location.trim();

            debug!("Location: {:?}", location);
            let values: Vec<Vec<u8>> = parse_value_string(values);
            for value in &values {
                if let Ok(s) = String::from_utf8(value.clone()) {
                    result_set.entry(location.to_owned()).or_default().insert(s);
                }
            }
        }
    }
    if verbose {
        debug!("Parsing results :\n");
        for (loc, vals) in &result_set {
            debug!("Location: {:?}", loc);
            for val in vals {
                debug!("Value: {:?}", val);
            }
        }
    }
    (format!("{result_set:?}"), result_set)
}

#[allow(clippy::panic_in_result_fn)]
async fn add_search_delete(
    key: String,
    cli_conf_path: &str,
    index_id: &Uuid,
    search_options: &SearchOptions,
) -> CliResult<()> {
    let test_conf = FindexClientConfig::load(Some(PathBuf::from(cli_conf_path)))?;
    let mut rest_client = FindexRestClient::new(test_conf)?;

    add(
        key.to_owned(),
        index_id,
        &search_options.dataset_path,
        &mut rest_client,
    )
    .await?;

    // make sure searching returns the expected results
    let search_results = search(key.to_owned(), index_id, search_options, &mut rest_client).await?;
    debug!("Search results (as string): {:?}", search_results);

    // parse Values to their unicode representation
    let (parsed_res, _) = parse_locations(&search_results, true);

    // check that the results are ok
    for expected_result in &search_options.expected_results {
        assert!(
            parsed_res.contains(expected_result),
            "Error after search. Expected {}, got {}",
            expected_result.as_str(),
            &search_results.as_str()
        );
    }

    delete(
        key.to_owned(),
        index_id,
        &search_options.dataset_path,
        &mut rest_client,
    )
    .await?;

    // make sure no results are returned after deletion
    let search_results = search(key, index_id, search_options, &mut rest_client).await?;
    for expected_result in &search_options.expected_results {
        assert!(
            !search_results.contains(expected_result),
            "Error after the post deletion search. Expected {expected_result}, got {search_results}",
        );
    }

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let key = "55223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_owned();

    // Search 2 entries in a small dataset. Expect 2 results.
    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };
    add_search_delete(
        key.to_owned(),
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        &search_options,
    )
    .await?;
    Ok(())
}

// todo(review) : this test yields the following error :
// Error: Default("ERROR: Memory(Conversion(\"insufficient bytes in a word to fit a value of length 146\"))\n")
// Does this mean we should adapt the test to the word size, or fix the conversion mechanism leaving the test intact ?
#[ignore]
#[tokio::test]
pub(crate) async fn test_findex_no_auth_huge_dataset() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let key = "44223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_owned();

    // Search 1 entry in a huge dataset
    let search_options = SearchOptions {
        dataset_path: HUGE_DATASET.into(),
        keywords: vec![
            "BDCQ.SEA1AA".to_owned(),
            "2011.06".to_owned(),
            "80078".to_owned(),
        ],
        expected_results: vec![
            "80078FNumber0Business Data Collection - BDCIndustry by employment variableFilled \
             jobsAgriculture, Forestry and FishingActual'"
                .to_owned(),
        ],
    };
    add_search_delete(
        key,
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        &search_options,
    )
    .await?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let mut owner_rest_client = FindexRestClient::new(owner_conf)?;
    let key = "33223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_owned();

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    let index_id = create_index_id(&mut owner_rest_client).await?;
    trace!("index_id: {index_id}");

    add_search_delete(key, &ctx.owner_client_conf_path, &index_id, &search_options).await?;
    Ok(())
}

#[allow(clippy::panic_in_result_fn, clippy::unwrap_used)]
#[tokio::test]
pub(crate) async fn test_findex_grant_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let mut owner_rest_client = FindexRestClient::new(owner_conf)?;
    let key = "22223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_owned();

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    let index_id = create_index_id(&mut owner_rest_client).await?;
    trace!("index_id: {index_id}");

    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let mut owner_rest_client = FindexRestClient::new(owner_conf)?;

    let user_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.user_client_conf_path)))?;
    let mut user_rest_client = FindexRestClient::new(user_conf)?;

    add(
        key.to_owned(),
        &index_id,
        SMALL_DATASET,
        &mut owner_rest_client,
    )
    .await?;

    // Grant read permission to the client
    grant_permission(
        &mut owner_rest_client,
        "user.client@acme.com".to_owned(),
        &index_id,
        Permission::Read,
    )
    .await?;

    // User can read...
    let search_results = search(
        key.to_owned(),
        &index_id,
        &search_options,
        &mut user_rest_client,
    )
    .await?;
    let (parsed_res, _) = parse_locations(&search_results, true);

    for expected_result in &search_options.expected_results {
        assert!(
            parsed_res.contains(expected_result),
            "Error during search : expected {expected_result}, got {parsed_res}",
        );
    }

    // ... but not write
    assert!(add(
        key.to_owned(),
        &index_id,
        SMALL_DATASET,
        &mut user_rest_client
    )
    .await
    .is_err());

    // Grant write permission
    grant_permission(
        &mut owner_rest_client,
        "user.client@acme.com".to_owned(),
        &index_id,
        Permission::Write,
    )
    .await?;

    let perm = list_permission(&mut owner_rest_client, "user.client@acme.com".to_owned()).await?;
    debug!("User permission: {:?}", perm);

    // User can read...
    let search_results = search(
        key.to_owned(),
        &index_id,
        &search_options,
        &mut user_rest_client,
    )
    .await?;
    let (parsed_res, _) = parse_locations(&search_results, true);
    for expected_result in &search_options.expected_results {
        assert!(
            parsed_res.contains(expected_result),
            "Error during search, Expected {expected_result}, Got {parsed_res}",
        );
    }

    // ... and write
    add(
        key.to_owned(),
        &index_id,
        SMALL_DATASET,
        &mut user_rest_client,
    )
    .await?;

    // Try to escalade privileges from `read` to `admin`
    grant_permission(
        &mut user_rest_client,
        "user.client@acme.com".to_owned(),
        &index_id,
        Permission::Admin,
    )
    .await
    .unwrap_err();

    revoke_permission(
        &mut owner_rest_client,
        "user.client@acme.com".to_owned(),
        &index_id,
    )
    .await?;

    let _search_results = search(key, &index_id, &search_options, &mut user_rest_client)
        .await
        .unwrap_err();

    Ok(())
}

#[allow(clippy::panic_in_result_fn)]
#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;
    let key = "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_owned();

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    assert!(add_search_delete(
        key,
        &ctx.user_client_conf_path,
        &Uuid::new_v4(),
        &search_options
    )
    .await
    .is_err());
    Ok(())
}
