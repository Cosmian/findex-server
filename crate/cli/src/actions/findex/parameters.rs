use clap::Parser;

use cosmian_kms_cli::{
    actions::symmetric::keys::create_key::{CreateKeyAction, SymmetricAlgorithm},
    reexport::cosmian_kms_client::KmsClient,
};
use uuid::Uuid;

use crate::error::result::CliResult;

#[derive(Clone, Parser, Debug, Default)]
#[clap(verbatim_doc_comment)]
pub struct FindexParameters {
    /// The user findex seed used (to insert, search and delete).
    /// The seed is a 32 bytes hex string.
    #[clap(required = false, short = 's', long, conflicts_with = "aes_xts_key_id")]
    pub seed_key_id: Option<String>,

    /// Either the seed or the KMS keys (HMAC and AES XTS keys) must be provided.
    /// The HMAC key ID used to encrypt the seed.
    #[clap(
        short = 'p',
        long,
        conflicts_with = "seed_key_id",
        requires = "aes_xts_key_id"
    )]
    pub hmac_key_id: Option<String>,

    /// The AES XTS key ID used to encrypt the index.
    #[clap(
        short = 'x',
        long,
        conflicts_with = "seed_key_id",
        requires = "hmac_key_id"
    )]
    pub aes_xts_key_id: Option<String>,

    /// The index ID
    #[clap(long, short = 'i')]
    pub index_id: Uuid,
}

impl FindexParameters {
    /// Returns a new `FindexParameters` instance with the KMS keys generated.
    /// By default, keys are generated inside KMS server and all cryptographic Findex operations is done usings KMS.
    ///
    /// # Errors
    /// This function will return an error if the KMS key generation fails.
    pub async fn new_with_encryption_keys(
        index_id: Uuid,
        kms_client: &KmsClient,
    ) -> CliResult<Self> {
        Ok(Self {
            seed_key_id: None,
            hmac_key_id: Some(Self::gen_hmac_key_id(kms_client).await?),
            aes_xts_key_id: Some(Self::gen_aes_xts_key_id(kms_client).await?),
            index_id,
        })
    }

    /// Returns a new `FindexParameters` instance with the seed and KMS keys generated.
    /// By default, keys are generated inside KMS server and all cryptographic Findex operations is done using KMS.
    ///
    /// # Errors
    /// This function will return an error if the KMS key generation fails.
    pub async fn new_for_client_side_encryption(
        index_id: Uuid,
        kms_client: &KmsClient,
    ) -> CliResult<Self> {
        Ok(Self {
            seed_key_id: Some(Self::gen_seed_id(kms_client).await?),
            hmac_key_id: None,
            aes_xts_key_id: None,
            index_id,
        })
    }

    pub(crate) async fn gen_seed_id(kms_client: &KmsClient) -> CliResult<String> {
        let uid = CreateKeyAction {
            number_of_bits: Some(256),
            algorithm: SymmetricAlgorithm::Aes,
            ..CreateKeyAction::default()
        }
        .run(kms_client)
        .await?;
        println!("Warning: This is the only time that this seed key ID will be printed. ID: {uid}");
        Ok(uid.to_string())
    }

    pub(crate) async fn get_hmac_key_id(&self, kms_client: &KmsClient) -> CliResult<String> {
        match &self.hmac_key_id {
            Some(id) => Ok(id.clone()),
            None => Self::gen_hmac_key_id(kms_client).await,
        }
    }

    pub(crate) async fn get_aes_xts_key_id(&self, kms_client: &KmsClient) -> CliResult<String> {
        match &self.aes_xts_key_id {
            Some(id) => Ok(id.clone()),
            None => Self::gen_aes_xts_key_id(kms_client).await,
        }
    }

    async fn gen_hmac_key_id(kms_client: &KmsClient) -> CliResult<String> {
        let uid = CreateKeyAction {
            number_of_bits: Some(256),
            algorithm: SymmetricAlgorithm::Sha3,
            ..CreateKeyAction::default()
        }
        .run(kms_client)
        .await?;
        println!("Warning: This is the only time that this HMAC key ID will be printed. ID: {uid}");
        Ok(uid.to_string())
    }

    pub(crate) async fn gen_aes_xts_key_id(kms_client: &KmsClient) -> CliResult<String> {
        let uid = CreateKeyAction {
            number_of_bits: Some(512),
            algorithm: SymmetricAlgorithm::Aes,
            ..CreateKeyAction::default()
        }
        .run(kms_client)
        .await?;
        println!(
            "Warning: This is the only time that this AES-XTS key ID will be printed. ID: {uid}"
        );
        Ok(uid.to_string())
    }
}
