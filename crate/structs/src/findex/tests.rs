#[allow(clippy::expect_used)]
#[cfg(test)]
mod findex_tests {

    use cosmian_crypto_core::{
        CsRng, Sampling,
        reexport::rand_core::{RngCore, SeedableRng},
    };
    use cosmian_findex::{ADDRESS_LENGTH, Address, WORD_LENGTH};

    use crate::findex::{
        addresses::Addresses, bindings::Bindings, guard::Guard, words::OptionalWords,
    };

    const SEED: [u8; 32] = [1_u8; 32]; // arbitrary seed for the RNG

    #[test]
    fn test_ser_deser_addresses() {
        let mut rng = CsRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let address2: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let addresses = Addresses(vec![address1, address2]);

        let serialized = addresses.serialize().expect("Serialization failed");
        let deserialized = Addresses::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(addresses, deserialized, "Addresses do not match",);
    }

    #[test]
    fn test_ser_deser_optional_words() {
        let mut rng = CsRng::from_seed(SEED);

        let mut word1 = [0_u8; WORD_LENGTH];
        let mut word2 = [0_u8; WORD_LENGTH];
        rng.fill_bytes(&mut word1[..]);
        rng.fill_bytes(&mut word2[..]);

        let optional_words: OptionalWords<{ WORD_LENGTH }> = OptionalWords(vec![None, Some(word1)]);

        let serialized = optional_words.serialize().expect("Serialization failed");
        let deserialized = OptionalWords::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(optional_words, deserialized, "Optional words do not match",);
    }

    #[test]
    fn test_ser_deser_guard() {
        let mut rng = CsRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let mut word = [0_u8; WORD_LENGTH];
        rng.fill_bytes(&mut word[..]);

        let guard_some: Guard<WORD_LENGTH> = Guard(address1, Some(word));
        let serialized_some = guard_some.serialize().expect("Serialization failed");
        let deserialized_some =
            Guard::deserialize(&serialized_some).expect("Deserialization failed");

        assert_eq!(
            guard_some, deserialized_some,
            "Guard with Some(word) does not match"
        );

        let address2: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let guard_none: Guard<WORD_LENGTH> = Guard(address2, None);

        let serialized_none = guard_none.serialize().expect("Serialization failed");
        let deserialized_none =
            Guard::deserialize(&serialized_none).expect("Deserialization failed");

        assert_eq!(
            guard_none, deserialized_none,
            "Guard with None does not match"
        );
    }

    #[test]
    fn test_ser_deser_bindings() {
        let mut rng = CsRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let address2: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let mut word1 = [0_u8; WORD_LENGTH];
        let mut word2 = [0_u8; WORD_LENGTH];
        rng.fill_bytes(&mut word1[..]);
        rng.fill_bytes(&mut word2[..]);

        let bindings = Bindings(vec![(address1, word1), (address2, word2)]);

        let serialized = bindings.serialize().expect("Serialization failed");
        let deserialized = Bindings::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(bindings, deserialized, "Bindings do not match");
    }
}
