use crate::{
    actions::findex::{
        insert_or_delete::InsertOrDeleteAction, parameters::FindexParameters, search::SearchAction,
    },
    error::result::CliResult,
};

use cosmian_findex::Value;
use cosmian_findex_client::FindexRestClient;
use std::{collections::HashSet, path::PathBuf};
use tracing::trace;
use uuid::Uuid;

pub(crate) async fn insert(
    seed: String,
    index_id: Uuid,
    dataset_path: &str,
    rest_client: &mut FindexRestClient,
) -> CliResult<()> {
    let res = InsertOrDeleteAction {
        findex_parameters: FindexParameters { seed, index_id },
        csv: PathBuf::from(dataset_path),
    }
    .insert(rest_client)
    .await?;

    trace!("Indexing of {} completed : {:?}", dataset_path, res);

    Ok(())
}

pub(crate) async fn delete(
    seed: String,
    index_id: Uuid,
    dataset_path: &str,
    rest_client: &mut FindexRestClient,
) -> CliResult<()> {
    InsertOrDeleteAction {
        findex_parameters: FindexParameters { seed, index_id },
        csv: PathBuf::from(dataset_path),
    }
    .delete(rest_client)
    .await?;

    trace!("Deletion of {} completed", dataset_path);

    Ok(())
}

#[derive(Clone)]
pub(crate) struct SearchOptions {
    /// The path to the CSV file containing the data to search in
    pub(crate) dataset_path: String,
    /// The keywords to search for
    pub(crate) keywords: Vec<String>,
    pub(crate) expected_results: HashSet<Value>,
}

pub(crate) async fn search(
    seed: String,
    index_id: Uuid,
    search_options: SearchOptions,
    rest_client: &mut FindexRestClient,
) -> CliResult<HashSet<Value>> {
    let res = SearchAction {
        findex_parameters: FindexParameters { seed, index_id },
        keyword: search_options.keywords,
    }
    .run(rest_client)
    .await?;

    trace!(
        "Search of {} completed : {:?}",
        search_options.dataset_path,
        res
    );

    Ok(res)
}
