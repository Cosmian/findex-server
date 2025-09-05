#[allow(clippy::expect_used)]
#[cfg(test)]
mod findex_tests {

    use cosmian_crypto_core::{
        CsRng, Sampling,
        reexport::rand_core::{RngCore, SeedableRng},
    };
    use cosmian_sse_memories::{ADDRESS_LENGTH, Address};

    use crate::{
        CUSTOM_WORD_LENGTH,
        findex::{
            addresses::Addresses,
            bindings::Bindings,
            guard::Guard,
            words::{OptionalWords, Word},
        },
    };

    const SEED: [u8; 32] = [1_u8; 32]; // arbitrary seed for the RNG
    const WORD_LENGTH: usize = CUSTOM_WORD_LENGTH;

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

        let guard_some: Guard<{ WORD_LENGTH }> = Guard(address1, Some(word));
        let serialized_some = guard_some.serialize().expect("Serialization failed");
        let deserialized_some =
            Guard::deserialize(&serialized_some).expect("Deserialization failed");

        assert_eq!(
            guard_some, deserialized_some,
            "Guard with Some(word) does not match"
        );

        let address2: Address<ADDRESS_LENGTH> = Address::random(&mut rng);
        let guard_none: Guard<{ WORD_LENGTH }> = Guard(address2, None);

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

    #[test]
    fn test_word_display() {
        // Test with a known byte array to verify base64 encoding
        let word_bytes = [1, 2, 3, 4, 5, 6, 7, 8];
        let word: Word<8> = Word::new(word_bytes);

        // Expected base64 encoding of [1, 2, 3, 4, 5, 6, 7, 8]
        let expected = "AQIDBAUGBwg=";
        assert_eq!(format!("{word}"), expected);
    }

    #[test]
    fn test_word_conversions() {
        let word_bytes = [1, 2, 3, 4, 5, 6, 7, 8];

        // Test From<[u8; N]>
        let word: Word<8> = Word::from(word_bytes);
        assert_eq!(word.as_bytes(), &word_bytes);

        // Test Into<[u8; N]>
        let back_to_bytes: [u8; 8] = word.into();
        assert_eq!(back_to_bytes, word_bytes);

        // Test AsRef<[u8; N]>
        let word2: Word<8> = Word::new(word_bytes);
        let as_ref_array: &[u8; 8] = word2.as_ref();
        assert_eq!(as_ref_array, &word_bytes);

        // Test AsRef<[u8]>
        let as_ref_slice: &[u8] = word2.as_ref();
        assert_eq!(as_ref_slice, &word_bytes);

        // Test Deref
        assert_eq!(*word2, word_bytes);
    }

    #[test]
    fn test_word_with_word_length() {
        let mut rng = CsRng::from_seed(SEED);
        let mut word_bytes = [0_u8; WORD_LENGTH];
        rng.fill_bytes(&mut word_bytes[..]);

        let word: Word<{ WORD_LENGTH }> = Word::new(word_bytes);

        // Test that display produces valid base64
        let display_str = format!("{word}");
        assert!(!display_str.is_empty());

        // Test round-trip conversion
        let back_to_bytes: [u8; WORD_LENGTH] = word.into_inner();
        assert_eq!(back_to_bytes, word_bytes);
    }

    #[test]
    fn test_word_real_example() {
        // Test with "Hello World!" which should produce "SGVsbG8gV29ybGQh" in base64
        let hello_bytes = [72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 33]; // "Hello World!"
        let word: Word<12> = Word::new(hello_bytes);

        let display_str = format!("{word}");
        let expected = "SGVsbG8gV29ybGQh"; // Base64 of "Hello World!"
        assert_eq!(display_str, expected);

        // Test round-trip
        let back_bytes: [u8; 12] = word.into();
        assert_eq!(back_bytes, hello_bytes);
    }
}
