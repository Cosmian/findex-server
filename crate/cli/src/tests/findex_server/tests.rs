use std::ops::Deref;

use cosmian_findex_client::{RestClient, RestClientConfig};
use cosmian_findex_structs::Uuids;
use cosmian_kms_cli::{
    actions::kms::symmetric::keys::create_key::CreateKeyAction,
    reexport::cosmian_kms_client::{
        KmsClient, KmsClientConfig, kmip_2_1::kmip_types::UniqueIdentifier,
        reexport::cosmian_kms_client_utils::symmetric_utils::DataEncryptionAlgorithm,
    },
};
use cosmian_logger::log_init;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use test_kms_server::start_default_test_kms_server;
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    actions::findex_server::{
        datasets::DeleteEntries, encrypt_and_index::EncryptAndIndexAction,
        findex::parameters::FindexParameters, permissions::CreateIndex,
        search_and_decrypt::SearchAndDecryptAction,
    },
    error::result::FindexCliResult,
};

const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";

#[derive(Clone)]
struct TestsCliContext {
    findex: RestClient,
    kms: KmsClient,
    search_options: SearchOptions,
    kek_id: Option<UniqueIdentifier>,
    index_id: Uuid,
}

#[derive(Clone)]
struct SearchOptions {
    dataset_path: String,
    keywords: Vec<String>,
    expected_results: String,
    expected_inserted_len: usize,
}

impl TestsCliContext {
    async fn new(
        findex_config: RestClientConfig,
        kms_config: KmsClientConfig,
        dataset: &str,
        keywords: Vec<String>,
        expected_results: &str,
        expected_len: usize,
    ) -> FindexCliResult<Self> {
        let kms = KmsClient::new_with_config(kms_config)?;
        let findex = RestClient::new(findex_config)?;
        let kek_id = Some(CreateKeyAction::default().run(kms.clone()).await?);
        let index_id = CreateIndex.run(findex.clone()).await?;
        trace!("index_id: {index_id}");

        Ok(Self {
            findex,
            kms,
            search_options: SearchOptions {
                dataset_path: dataset.into(),
                keywords,
                expected_results: expected_results.to_string(),
                expected_inserted_len: expected_len,
            },
            kek_id,
            index_id,
        })
    }

    async fn run_test_sequence(&self) -> FindexCliResult<()> {
        let findex_parameters =
            FindexParameters::new(self.index_id, self.kms.clone(), true, Some(1)).await?;

        // Index
        info!("==> Indexing dataset");
        let uuids = self.index(&findex_parameters).await?;

        // Search
        info!("==> Searching for keywords");
        let results = self.search(&findex_parameters).await?;
        assert!(
            results
                .iter()
                .any(|r| r.contains(&self.search_options.expected_results))
        );

        // Delete
        info!("==> Deleting entries");
        self.delete(&uuids).await?;

        // Verify deletion
        info!("==> Verifying deletion");
        let results = self.search(&findex_parameters).await?;
        assert!(results.is_empty());

        Ok(())
    }

    async fn index(&self, params: &FindexParameters) -> FindexCliResult<Uuids> {
        let action = EncryptAndIndexAction {
            findex_parameters: params.clone(),
            csv: self.search_options.dataset_path.clone().into(),
            key_encryption_key_id: self.kek_id.as_ref().map(ToString::to_string),
            data_encryption_key_id: None,
            data_encryption_algorithm: DataEncryptionAlgorithm::AesGcm,
            nonce: None,
            authentication_data: None,
        };
        let uuids = action.run(self.findex.clone(), self.kms.clone()).await?;
        assert_eq!(uuids.len(), self.search_options.expected_inserted_len);
        Ok(uuids)
    }

    async fn search(&self, params: &FindexParameters) -> FindexCliResult<Vec<String>> {
        SearchAndDecryptAction {
            findex_parameters: params.clone(),
            key_encryption_key_id: self.kek_id.as_ref().map(ToString::to_string),
            data_encryption_key_id: None,
            data_encryption_algorithm: DataEncryptionAlgorithm::AesGcm,
            keyword: self.search_options.keywords.clone(),
            authentication_data: None,
        }
        .run(self.findex.clone(), &self.kms)
        .await
    }

    async fn delete(&self, uuids: &Uuids) -> FindexCliResult<String> {
        DeleteEntries {
            index_id: self.index_id,
            uuids: uuids.deref().clone(),
        }
        .run(self.findex.clone())
        .await
    }
}

#[tokio::test]
async fn test_encrypt_and_index_no_auth() -> FindexCliResult<()> {
    log_init(None);
    let findex_ctx = start_default_test_findex_server().await;
    let kms_ctx = start_default_test_kms_server().await;

    let ctx = TestsCliContext::new(
        findex_ctx.owner_client_conf.clone(),
        kms_ctx.owner_client_config.clone(),
        SMALL_DATASET,
        vec!["Southborough".to_owned()],
        "States9686",
        10,
    )
    .await?;
    ctx.run_test_sequence().await
}

#[tokio::test]
async fn test_encrypt_and_index_cert_auth() -> FindexCliResult<()> {
    log_init(None);
    // log_init(Some("info,cosmian_kms_server=debug"));

    let findex_ctx = start_default_test_findex_server_with_cert_auth().await;
    let kms_ctx = start_default_test_kms_server().await;

    let ctx = TestsCliContext::new(
        findex_ctx.owner_client_conf.clone(),
        kms_ctx.owner_client_config.clone(),
        SMALL_DATASET,
        vec!["Southborough".to_owned()],
        "States9686",
        10,
    )
    .await?;
    ctx.run_test_sequence().await
}

#[ignore]
#[tokio::test]
async fn test_encrypt_and_index_huge() -> FindexCliResult<()> {
    log_init(None);

    let findex_ctx = start_default_test_findex_server_with_cert_auth().await;
    let kms_ctx = start_default_test_kms_server().await;

    let ctx = TestsCliContext::new(
        findex_ctx.owner_client_conf.clone(),
        kms_ctx.owner_client_config.clone(),
        HUGE_DATASET,
        vec![
            "BDCQ.SEA1AA".to_owned(),
            "2011.06".to_owned(),
            "80078".to_owned(),
        ],
        "BDCQ.SEA1AA2011.0680078FNumber0Business Data Collection",
        23350,
    )
    .await?;
    ctx.run_test_sequence().await
}
