use cosmian_findex::{ADDRESS_LENGTH, Address, MemoryADT};
use cosmian_kms_cli::reexport::cosmian_kms_client::{
    KmsClient,
    cosmian_kmip::kmip_0::{
        kmip_messages::{ResponseMessage, ResponseMessageBatchItemVersioned},
        kmip_types::ResultStatusEnumeration,
    },
};
use tracing::trace;

use crate::{ClientError, ClientResult};

/// The encryption layers is built on top of an encrypted memory implementing the `MemoryADT` and
/// exposes a plaintext virtual memory interface implementing the `MemoryADT`.
///
/// This type is thread-safe.
#[derive(Clone)]
pub struct KmsEncryptionLayer<
    const WORD_LENGTH: usize,
    Memory: Send + Sync + Clone + MemoryADT<Address = Address<ADDRESS_LENGTH>>,
> {
    pub(crate) kms_client: KmsClient,
    pub(crate) hmac_key_id: String,
    pub(crate) aes_xts_key_id: String,
    pub(crate) mem: Memory,
}

impl<
    const WORD_LENGTH: usize,
    Memory: Send + Sync + Clone + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; WORD_LENGTH]>,
> KmsEncryptionLayer<WORD_LENGTH, Memory>
{
    /// Instantiates a new memory encryption layer.
    pub const fn new(
        kms_client: KmsClient,
        hmac_key_id: String,
        aes_xts_key_id: String,
        mem: Memory,
    ) -> Self {
        Self {
            kms_client,
            hmac_key_id,
            aes_xts_key_id,
            mem,
        }
    }

    fn extract_words(message_response: &ResponseMessage) -> ClientResult<Vec<[u8; WORD_LENGTH]>> {
        let valid_item = |item: &ResponseMessageBatchItemVersioned| {
            if let ResponseMessageBatchItemVersioned::V21(item) = item {
                item.result_status == ResultStatusEnumeration::Success
            } else {
                false
            }
        };
        if message_response.batch_item.iter().all(valid_item) {
            message_response
                .extract_items_data()?
                .iter()
                .map(|c| {
                    <[u8; WORD_LENGTH]>::try_from(c.as_slice())
                        .map_err(|e| ClientError::Default(format!("wrong slice length: {e}")))
                })
                .collect()
        } else {
            Err(ClientError::Default(
                "One or more operations failed in the batch".to_owned(),
            ))
        }
    }

    /// Compute multiple HMAC on given memory addresses.
    pub(crate) async fn batch_permute<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Memory::Address>,
    ) -> ClientResult<Vec<Memory::Address>> {
        let tokens = self
            .kms_client
            .message(self.build_mac_message_request(addresses)?)
            .await?
            .extract_items_data()?
            .into_iter()
            .map(|mac| {
                // Truncate to the first ADDRESS_LENGTH bytes
                mac.get(0..ADDRESS_LENGTH)
                    .ok_or_else(|| {
                        ClientError::Default(format!(
                            "Could not extract first {ADDRESS_LENGTH} bytes of the computed HMAC"
                        ))
                    })?
                    .try_into()
                    .map(|array: [u8; ADDRESS_LENGTH]| Address::from(array))
                    .map_err(|e| ClientError::Default(format!("Conversion error: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        trace!("hmac: tokens: {:?}", tokens);
        Ok(tokens)
    }

    /// Bulk encrypts the given words using AES-XTS-512 and the given memory addresses as tweak.
    pub(crate) async fn batch_encrypt<'a>(
        &self,
        bindings: impl Iterator<Item = (&'a Memory::Address, &'a [u8; WORD_LENGTH])>,
    ) -> ClientResult<Vec<[u8; WORD_LENGTH]>> {
        Self::extract_words(
            &self
                .kms_client
                .message(self.build_encrypt_message_request(bindings)?)
                .await?,
        )
    }

    /// Decrypts these ciphertexts using the given addresses as tweak.
    pub(crate) async fn batch_decrypt<'a>(
        &self,
        bindings: impl Iterator<Item = (&'a Memory::Address, &'a [u8; WORD_LENGTH])>,
    ) -> ClientResult<Vec<[u8; WORD_LENGTH]>> {
        Self::extract_words(
            &self
                .kms_client
                .message(self.build_decrypt_message_request(bindings)?)
                .await?,
        )
    }
}
