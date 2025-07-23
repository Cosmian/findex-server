use cosmian_findex::KEY_LENGTH;
use cosmian_kms_cli::reexport::{
    cosmian_kms_client::{KmsClient, kmip_2_1::kmip_operations::Get},
    cosmian_kms_crypto::reexport::cosmian_crypto_core::{Secret, reexport::zeroize::Zeroizing},
};

use crate::error::result::{FindexCliResult, FindexCliResultHelper};

pub mod findex_instance;
pub mod insert_or_delete;
pub mod parameters;
pub mod search;

/// Retrieve the key bytes of a key from KMS.
///
/// # Errors
/// Fails if the key if KMS client fails
pub async fn retrieve_key_from_kms(
    key_id: &str,
    kms_client: KmsClient,
) -> FindexCliResult<Secret<KEY_LENGTH>> {
    // Handle the case where seed_kms_id is set
    let mut secret = Zeroizing::new([0_u8; KEY_LENGTH]);
    secret.copy_from_slice(
        &kms_client
            .get(Get::from(key_id))
            .await?
            .object
            .key_block()?
            .symmetric_key_bytes()
            .context("findex::retrieve_key_from_kms")?,
    );
    Ok(Secret::from_unprotected_bytes(&mut secret))
}
