use crate::{FindexClientError, FindexRestClient};
use cosmian_crypto_core::CsRng;
use cosmian_findex::{Findex, Secret, Value};
use cosmian_findex_server::database::redis::{decode_fn, encode_fn, WORD_LENGTH};
use tracing::trace;

#[allow(clippy::future_not_send)]
pub async fn instantiate_findex(
    rest_client: FindexRestClient,
) -> Result<
    Findex<{ WORD_LENGTH }, Value, std::convert::Infallible, FindexRestClient>,
    FindexClientError,
> {
    let mut rng = CsRng::from_entropy();
    let seed = Secret::random(&mut rng);
    trace!("Instantiating Findex rest client with seed: {:?}", seed); // TODO(review) : should we log the seed?

    let res = Findex::new(seed, rest_client, encode_fn::<WORD_LENGTH, _>, decode_fn);
    Ok(res)
}
