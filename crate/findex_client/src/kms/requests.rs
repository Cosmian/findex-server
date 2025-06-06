use cosmian_findex_memories::reexport::cosmian_findex::{ADDRESS_LENGTH, Address, MemoryADT};
use cosmian_kms_cli::reexport::cosmian_kms_client::{
    cosmian_kmip::kmip_0::{
        kmip_messages::{RequestMessage, RequestMessageBatchItemVersioned, RequestMessageHeader},
        kmip_types::{BlockCipherMode, HashingAlgorithm, ProtocolVersion},
    },
    kmip_2_1::{
        kmip_messages::RequestMessageBatchItem,
        kmip_operations::{Decrypt, Encrypt, MAC, Operation},
        kmip_types::{CryptographicAlgorithm, CryptographicParameters, UniqueIdentifier},
        requests::encrypt_request,
    },
};

use super::KmsEncryptionLayer;
use crate::ClientResult;

impl<
    const WORD_LENGTH: usize,
    Memory: Send + Sync + Clone + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; WORD_LENGTH]>,
> KmsEncryptionLayer<WORD_LENGTH, Memory>
{
    fn build_message_request(
        items: Vec<RequestMessageBatchItemVersioned>,
    ) -> ClientResult<RequestMessage> {
        let items_number = i32::try_from(items.len())?;
        Ok(RequestMessage {
            request_header: RequestMessageHeader {
                protocol_version: ProtocolVersion {
                    protocol_version_major: 2,
                    protocol_version_minor: 1,
                },
                maximum_response_size: Some(9999),
                batch_count: items_number,
                ..Default::default()
            },
            batch_item: items,
        })
    }

    fn build_mac_request(&self, data: Vec<u8>) -> MAC {
        MAC {
            unique_identifier: Some(UniqueIdentifier::TextString(self.hmac_key_id.clone())),
            cryptographic_parameters: Some(CryptographicParameters {
                hashing_algorithm: Some(HashingAlgorithm::SHA3256),
                ..CryptographicParameters::default()
            }),
            data: Some(data),
            ..Default::default()
        }
    }

    pub(crate) fn build_mac_message_request<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Memory::Address>,
    ) -> ClientResult<RequestMessage> {
        let items = addresses
            .map(|address| {
                RequestMessageBatchItemVersioned::V21(RequestMessageBatchItem::new(Operation::MAC(
                    self.build_mac_request(address.to_vec()),
                )))
            })
            .collect();
        Self::build_message_request(items)
    }

    fn build_encrypt_request(&self, plaintext: Vec<u8>, nonce: Vec<u8>) -> ClientResult<Encrypt> {
        Ok(encrypt_request(
            &self.aes_xts_key_id,
            None,
            plaintext,
            Some(nonce),
            None,
            Some(CryptographicParameters {
                cryptographic_algorithm: Some(CryptographicAlgorithm::AES),
                block_cipher_mode: Some(BlockCipherMode::XTS),
                ..CryptographicParameters::default()
            }),
        )?)
    }

    pub(crate) fn build_encrypt_message_request<'a>(
        &self,
        bindings: impl Iterator<Item = (&'a Memory::Address, &'a [u8; WORD_LENGTH])>,
    ) -> ClientResult<RequestMessage> {
        let items = bindings
            .map(|(address, word)| {
                self.build_encrypt_request(word.to_vec(), address.to_vec())
                    .map(|encrypt_request| {
                        RequestMessageBatchItemVersioned::V21(RequestMessageBatchItem::new(
                            Operation::Encrypt(encrypt_request),
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::build_message_request(items)
    }

    fn build_decrypt_request(&self, ciphertext: Vec<u8>, nonce: Vec<u8>) -> Decrypt {
        Decrypt {
            unique_identifier: Some(UniqueIdentifier::TextString(self.aes_xts_key_id.clone())),
            cryptographic_parameters: Some(CryptographicParameters {
                cryptographic_algorithm: Some(CryptographicAlgorithm::AES),
                block_cipher_mode: Some(BlockCipherMode::XTS),
                ..CryptographicParameters::default()
            }),
            data: Some(ciphertext),
            i_v_counter_nonce: Some(nonce),
            ..Default::default()
        }
    }

    pub(crate) fn build_decrypt_message_request<'a>(
        &self,
        bindings: impl Iterator<Item = (&'a Memory::Address, &'a [u8; WORD_LENGTH])>,
    ) -> ClientResult<RequestMessage> {
        let items = bindings
            .map(|(address, word)| {
                RequestMessageBatchItemVersioned::V21(RequestMessageBatchItem::new(
                    Operation::Decrypt(self.build_decrypt_request(word.to_vec(), address.to_vec())),
                ))
            })
            .collect::<Vec<_>>();
        Self::build_message_request(items)
    }
}
