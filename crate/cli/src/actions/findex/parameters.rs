use clap::Parser;

use cosmian_kms_cli::{
    actions::symmetric::keys::create_key::{CreateKeyAction, SymmetricAlgorithm},
    reexport::cosmian_kms_client::KmsClient,
};
use uuid::Uuid;

use crate::error::{result::CliResult, CliError};

use super::findex_instance::FindexKeys;

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
    #[allow(clippy::as_conversions)]
    /// Instantiates the Findex parameters.
    ///
    /// # Errors
    /// - if the keys cannot be generate via the KMS client
    pub async fn new(
        index_id: Uuid,
        kms_client: &KmsClient,
        server_side_encryption: bool,
    ) -> CliResult<Self> {
        async fn generate_key(
            kms_client: &KmsClient,
            bits: u32,
            algorithm: SymmetricAlgorithm,
            key_type: &str,
        ) -> CliResult<String> {
            let uid = CreateKeyAction {
                number_of_bits: Some(bits as usize),
                algorithm,
                ..CreateKeyAction::default()
            }
            .run(kms_client)
            .await?;
            println!(
            "Warning: This is the only time that this {key_type} key ID will be printed. ID: {uid}"
        );
            Ok(uid.to_string())
        }

        if server_side_encryption {
            Ok(Self {
                seed_key_id: None,
                hmac_key_id: Some(
                    generate_key(kms_client, 256, SymmetricAlgorithm::Sha3, "HMAC").await?,
                ),
                aes_xts_key_id: Some(
                    generate_key(kms_client, 512, SymmetricAlgorithm::Aes, "AES-XTS").await?,
                ),
                index_id,
            })
        } else {
            Ok(Self {
                seed_key_id: Some(
                    generate_key(kms_client, 256, SymmetricAlgorithm::Aes, "seed").await?,
                ),
                hmac_key_id: None,
                aes_xts_key_id: None,
                index_id,
            })
        }
    }
    /// Instantiates the Findex keys.
    /// If a seed key is provided, the client side encryption is used.
    /// Otherwise, the KMS server-side encryption is used.
    ///
    /// # Errors
    /// - if no key id is provided
    pub fn instantiate_keys(self) -> CliResult<FindexKeys> {
        match (self.seed_key_id, self.hmac_key_id, self.aes_xts_key_id) {
            (Some(seed_key_id), None, None) => Ok(FindexKeys::ClientSideEncryption {
                seed_key_id,
                index_id: self.index_id,
            }),
            (None, Some(hmac_key_id), Some(aes_xts_key_id)) => {
                Ok(FindexKeys::ServerSideEncryption {
                    hmac_key_id,
                    aes_xts_key_id,
                    index_id: self.index_id,
                })
            }
            _ => Err(CliError::Default(
                "Either the seed or the KMS keys (HMAC and AES XTS keys) must be provided."
                    .to_owned(),
            )),
        }
    }
}
