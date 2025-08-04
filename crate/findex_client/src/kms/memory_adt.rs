use std::iter::once;

use cosmian_memories::{ADDRESS_LENGTH, Address, MemoryADT};
use tracing::trace;

use super::KmsEncryptionLayer;
use crate::ClientError;

impl<
    const WORD_LENGTH: usize,
    Memory: Send + Sync + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; WORD_LENGTH]>,
> MemoryADT for KmsEncryptionLayer<WORD_LENGTH, Memory>
{
    type Address = Address<ADDRESS_LENGTH>;
    type Error = ClientError;
    type Word = [u8; WORD_LENGTH];

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, Self::Error> {
        // Cryptographic operations being delegated to the KMS, it is better to
        // perform them in batch. Since permuted addresses are used as tweak in
        // the AES-XTS encryption of the words, two batches are required. A
        // third and final call is required to decrypt the guard value returned
        // by the memory.

        trace!("guarded_write: {guard:?}, {bindings:?}");

        let permuted_addresses = self
            .batch_permute(bindings.iter().map(|(a, _)| a).chain([&guard.0]))
            .await?;

        let encrypted_words = self
            .batch_encrypt(
                permuted_addresses
                    .iter()
                    .zip(bindings.iter().map(|(_, w)| w).chain(guard.1.iter())),
            )
            .await?;

        let encrypted_guard = (
            *permuted_addresses.get(bindings.len()).ok_or_else(|| {
                ClientError::Default("no permuted guard address found".to_owned())
            })?,
            encrypted_words.get(bindings.len()).copied(),
        );

        let encrypted_bindings = permuted_addresses
            .into_iter()
            .zip(encrypted_words)
            .take(bindings.len())
            .collect::<Vec<_>>();

        let permuted_ag = encrypted_guard.0;

        // Perform the actual call to the memory.
        let encrypted_wg_cur = self
            .mem
            .guarded_write(encrypted_guard, encrypted_bindings)
            .await
            .map_err(|e| ClientError::Default(format!("Memory error: {e}")))?;

        let wg_cur = match encrypted_wg_cur {
            Some(ctx) => Some(
                *self
                    .batch_decrypt(once((&permuted_ag, &ctx)))
                    .await?
                    .first()
                    .ok_or_else(|| ClientError::Default("No plaintext found".to_owned()))?,
            ),
            None => None,
        };

        trace!("guarded_write: current guard word: {wg_cur:?}");

        Ok(wg_cur)
    }

    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<Self::Word>>, Self::Error> {
        trace!("batch_read: addresses: {:?}", addresses);

        let permuted_addresses = self.batch_permute(addresses.iter()).await?;

        let encrypted_words = self
            .mem
            .batch_read(permuted_addresses.clone())
            .await
            .map_err(|e| ClientError::Default(format!("Memory error: {e}")))?;

        if permuted_addresses.len() != encrypted_words.len() {
            return Err(ClientError::Default(format!(
                "incorrect number of words: expected {}, but {} were given",
                permuted_addresses.len(),
                encrypted_words.len()
            )));
        }

        // None values need to be filtered out to compose with batch_decrypt.
        // However, their positions shall not be lost.
        let some_encrypted_words = encrypted_words
            .into_iter()
            .enumerate()
            .filter_map(|(i, w)| w.map(|w| (i, w)))
            .collect::<Vec<_>>();
        trace!(
            "batch_read: some_encrypted_words: {:?}",
            some_encrypted_words
        );
        if some_encrypted_words.is_empty() {
            return Ok(vec![None; addresses.len()]);
        }

        let some_words = self
            .batch_decrypt(
                // Since indexes are produced using encrypted_words and the
                // above check guarantees its length is equal to the length of
                // permuted_addresses, the following indexing is guaranteed to
                // be in range.
                #[expect(clippy::indexing_slicing)]
                some_encrypted_words
                    .iter()
                    .map(|(i, w)| (&permuted_addresses[*i], w)),
            )
            .await?;

        // Replace the None values in the list of decrypted words at the same
        // position as in the list of encrypted words.
        let mut pos = some_encrypted_words.into_iter().map(|(i, _)| i).peekable();
        let mut words = Vec::with_capacity(addresses.len());
        let mut some_words = some_words.into_iter();
        for i in 0..addresses.len() {
            if Some(&i) == pos.peek() {
                pos.next();
                words.push(some_words.next());
            } else {
                words.push(None);
            }
        }

        trace!("batch_read: words: {:?}", words);

        Ok(words)
    }
}

#[cfg(test)]
#[expect(clippy::panic_in_result_fn)]
mod tests {
    use std::sync::Arc;

    use cosmian_findex_structs::CUSTOM_WORD_LENGTH;
    use cosmian_kms_cli::reexport::{
        cosmian_kms_client::{
            KmsClient, KmsClientConfig,
            kmip_2_1::{
                extra::tagging::EMPTY_TAGS, kmip_types::CryptographicAlgorithm,
                requests::symmetric_key_create_request,
            },
        },
        cosmian_kms_crypto::reexport::cosmian_crypto_core::{
            CsRng, Sampling, reexport::rand_core::SeedableRng,
        },
    };
    use cosmian_logger::log_init;
    use cosmian_memories::{
        InMemory,
        test_utils::{
            gen_seed, test_guarded_write_concurrent, test_rw_same_address,
            test_single_write_and_read, test_wrong_guard,
        },
    };
    use test_kms_server::start_default_test_kms_server;
    use tokio::task;

    use super::*;
    use crate::ClientResult;

    async fn create_test_layer<const WORD_LENGTH: usize>(
        kms_config: KmsClientConfig,
    ) -> ClientResult<
        KmsEncryptionLayer<WORD_LENGTH, InMemory<Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH]>>,
    > {
        let memory = InMemory::default();
        let kms_client = KmsClient::new_with_config(kms_config)?;

        let k_p = kms_client
            .create(symmetric_key_create_request(
                None,
                256,
                CryptographicAlgorithm::SHAKE256,
                EMPTY_TAGS,
                false,
                None,
            )?)
            .await?
            .unique_identifier
            .to_string();

        let k_xts = kms_client
            .create(symmetric_key_create_request(
                None,
                512,
                CryptographicAlgorithm::AES,
                EMPTY_TAGS,
                false,
                None,
            )?)
            .await?
            .unique_identifier
            .to_string();

        Ok(KmsEncryptionLayer::<WORD_LENGTH, _>::new(
            kms_client, k_p, k_xts, memory,
        ))
    }

    #[tokio::test]
    #[expect(clippy::panic_in_result_fn, clippy::unwrap_used)]
    async fn test_adt_encrypt_decrypt() -> ClientResult<()> {
        let mut rng = CsRng::from_entropy();
        let tok = Address::<ADDRESS_LENGTH>::random(&mut rng);
        let ptx = [1; CUSTOM_WORD_LENGTH];

        let ctx = start_default_test_kms_server().await;
        let layer = create_test_layer(ctx.owner_client_config.clone()).await?;

        let layer = Arc::new(layer);
        let mut handles = vec![];

        handles.push(task::spawn(async move {
            for _ in 0..1_000 {
                let ctx = layer.batch_encrypt(once((&tok, &ptx))).await?.remove(0);
                let res = layer.batch_decrypt(once((&tok, &ctx))).await?.remove(0);
                assert_eq!(ptx, res);
                assert_eq!(ptx.len(), res.len());
            }
            Ok::<(), ClientError>(())
        }));

        for handle in handles {
            handle.await.unwrap()?;
        }
        Ok(())
    }

    /// Ensures a transaction can express a vector push operation:
    /// - the counter is correctly incremented and all values are written;
    /// - using the wrong value in the guard fails the operation and returns the current value.
    #[tokio::test]
    async fn test_single_vector_push() -> ClientResult<()> {
        log_init(None);
        let mut rng = CsRng::from_entropy();

        let ctx = start_default_test_kms_server().await;
        let layer = create_test_layer(ctx.owner_client_config.clone()).await?;

        let header_addr = Address::<ADDRESS_LENGTH>::random(&mut rng);

        assert_eq!(
            layer
                .guarded_write(
                    (header_addr, None),
                    vec![(header_addr, [2; CUSTOM_WORD_LENGTH]),]
                )
                .await?,
            None
        );

        assert_eq!(
            vec![Some([2; CUSTOM_WORD_LENGTH])],
            layer.batch_read(vec![header_addr,]).await?
        );
        Ok(())
    }

    /// Ensures a transaction can express a vector push operation:
    /// - the counter is correctly incremented and all values are written;
    /// - using the wrong value in the guard fails the operation and returns the current value.
    #[tokio::test]
    async fn test_twice_vector_push() -> ClientResult<()> {
        log_init(None);
        let mut rng = CsRng::from_entropy();
        let ctx = start_default_test_kms_server().await;
        let layer = create_test_layer(ctx.owner_client_config.clone()).await?;

        let header_addr = Address::<ADDRESS_LENGTH>::random(&mut rng);

        let val_addr_1 = Address::<ADDRESS_LENGTH>::random(&mut rng);

        assert_eq!(
            layer
                .guarded_write(
                    (header_addr, None),
                    vec![
                        (header_addr, [2; CUSTOM_WORD_LENGTH]),
                        (val_addr_1, [1; CUSTOM_WORD_LENGTH]),
                    ]
                )
                .await?,
            None
        );

        assert_eq!(
            vec![Some([2; CUSTOM_WORD_LENGTH]), Some([1; CUSTOM_WORD_LENGTH])],
            layer.batch_read(vec![header_addr, val_addr_1,]).await?
        );
        Ok(())
    }

    /// Ensures a transaction can express a vector push operation:
    /// - the counter is correctly incremented and all values are written;
    /// - using the wrong value in the guard fails the operation and returns the current value.
    #[tokio::test]
    async fn test_vector_push() -> ClientResult<()> {
        log_init(None);
        let mut rng = CsRng::from_entropy();
        let ctx = start_default_test_kms_server().await;
        let layer = create_test_layer(ctx.owner_client_config.clone()).await?;

        let header_addr = Address::<ADDRESS_LENGTH>::random(&mut rng);

        let val_addr_1 = Address::<ADDRESS_LENGTH>::random(&mut rng);
        let val_addr_2 = Address::<ADDRESS_LENGTH>::random(&mut rng);
        let val_addr_3 = Address::<ADDRESS_LENGTH>::random(&mut rng);
        let val_addr_4 = Address::<ADDRESS_LENGTH>::random(&mut rng);

        assert_eq!(
            layer
                .guarded_write(
                    (header_addr, None),
                    vec![
                        (header_addr, [2; CUSTOM_WORD_LENGTH]),
                        (val_addr_1, [1; CUSTOM_WORD_LENGTH]),
                        (val_addr_2, [1; CUSTOM_WORD_LENGTH])
                    ]
                )
                .await?,
            None
        );

        assert_eq!(
            layer
                .guarded_write(
                    (header_addr, None),
                    vec![
                        (header_addr, [2; CUSTOM_WORD_LENGTH]),
                        (val_addr_1, [3; CUSTOM_WORD_LENGTH]),
                        (val_addr_2, [3; CUSTOM_WORD_LENGTH])
                    ]
                )
                .await?,
            Some([2; CUSTOM_WORD_LENGTH])
        );

        assert_eq!(
            layer
                .guarded_write(
                    (header_addr, Some([2; CUSTOM_WORD_LENGTH])),
                    vec![
                        (header_addr, [4; CUSTOM_WORD_LENGTH]),
                        (val_addr_3, [2; CUSTOM_WORD_LENGTH]),
                        (val_addr_4, [2; CUSTOM_WORD_LENGTH])
                    ]
                )
                .await?,
            Some([2; CUSTOM_WORD_LENGTH])
        );

        assert_eq!(
            vec![
                Some([4; CUSTOM_WORD_LENGTH]),
                Some([1; CUSTOM_WORD_LENGTH]),
                Some([1; CUSTOM_WORD_LENGTH]),
                Some([2; CUSTOM_WORD_LENGTH]),
                Some([2; CUSTOM_WORD_LENGTH])
            ],
            layer
                .batch_read(vec![
                    header_addr,
                    val_addr_1,
                    val_addr_2,
                    val_addr_3,
                    val_addr_4
                ])
                .await?
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_sequential_read_write() -> ClientResult<()> {
        log_init(None);
        let ctx = start_default_test_kms_server().await;
        let memory = create_test_layer(ctx.owner_client_config.clone()).await?;

        test_single_write_and_read::<CUSTOM_WORD_LENGTH, _>(&memory, gen_seed()).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_sequential_wrong_guard() -> ClientResult<()> {
        let ctx = start_default_test_kms_server().await;
        let memory = create_test_layer(ctx.owner_client_config.clone()).await?;
        test_wrong_guard::<CUSTOM_WORD_LENGTH, _>(&memory, gen_seed()).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_sequential_same_address() -> ClientResult<()> {
        let ctx = start_default_test_kms_server().await;
        let memory = create_test_layer(ctx.owner_client_config.clone()).await?;
        test_rw_same_address::<CUSTOM_WORD_LENGTH, _>(&memory, gen_seed()).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_read_write() -> ClientResult<()> {
        log_init(None);
        let ctx = start_default_test_kms_server().await;
        let memory = create_test_layer(ctx.owner_client_config.clone()).await?;
        test_guarded_write_concurrent::<
            CUSTOM_WORD_LENGTH,
            _,
            cosmian_findex::reexport::tokio::TokioSpawner,
        >(&memory, gen_seed(), Some(100))
        .await;
        Ok(())
    }
}
