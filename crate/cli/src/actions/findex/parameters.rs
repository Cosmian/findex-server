use clap::Parser;

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
