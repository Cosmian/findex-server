/// this module contains serialization and deserialization functions needed for the Findex server
use crate::StructsError;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{Address, ADDRESS_LENGTH};
use cosmian_findex_config::WORD_LENGTH;

type SerializationResult<R> = Result<R, StructsError>;

#[derive(Debug, Clone, PartialEq)]
pub struct Addresses(Vec<Address<ADDRESS_LENGTH>>);

// Serialization functions
impl Addresses {
    pub fn serialize(addresses: Addresses) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(addresses.0.len());
        ser.write_leb128_u64(addresses.0.len() as u64)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for adr in addresses.0.iter() {
            ser.write_array(adr.as_ref())
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    pub fn deserialize(data: Vec<u8>) -> SerializationResult<Addresses> {
        let mut de = Deserializer::new(&data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);
        for _ in 0..length {
            let key: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            items.push(key);
        }
        Ok(Addresses(items))
    }
}
fn ser_option_word<const WORD_LENGTH: usize>(
    ser: &mut Serializer,
    word: &Option<[u8; WORD_LENGTH]>,
) -> SerializationResult<usize> {
    match word {
        Some(w) => {
            ser.write_leb128_u64(1)?;
            ser.write_array(w)
        }
        None => ser.write_leb128_u64(0),
    }
    .map_err(|e| StructsError::SerializationError(e.to_string()))
}

fn read_optional_word<const WORD_LENGTH: usize>(
    de: &mut Deserializer,
) -> SerializationResult<Option<[u8; WORD_LENGTH]>> {
    let flag = <usize>::try_from(de.read_leb128_u64()?)?;
    match flag {
        0 => Ok(None),
        1 => {
            let word: [u8; WORD_LENGTH] = de.read_array()?;
            Ok(Some(word))
        }
        _ => Err(StructsError::DeserializationError(
            "Invalid value for serialized option flag".to_string(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionalWords<const WORD_LENGTH: usize>(Vec<Option<[u8; WORD_LENGTH]>>);

impl<const WORD_LENGTH: usize> OptionalWords<WORD_LENGTH> {
    /// Serializes the `OptionalWords` instance into a vector of bytes.
    ///
    /// The serialization protocol is as follows:
    /// 1. The length of the vector is serialized as a LEB128-encoded u64.
    /// 2. Each element in the vector is serialized as follows:
    ///    - If the element is `Some`, a `1` is written followed by the byte array.
    ///    - If the element is `None`, a `0` is written.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if any step of the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len() as u64)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for word in self.0.iter() {
            ser_option_word(&mut ser, word)
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    pub fn deserialize(data: Vec<u8>) -> SerializationResult<Self> {
        let mut de = Deserializer::new(&data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);

        for _ in 0..length {
            let word = read_optional_word::<WORD_LENGTH>(&mut de)?;
            items.push(word);
        }
        Ok(OptionalWords(items))
    }
}

#[derive(Debug)]
struct Guard<const WORD_LENGTH: usize>(Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>);

// #[derive(Debug)]
// struct Tasks<const WORD_LENGTH: usize>(Vec<(Address, [u8; WORD_LENGTH])>);

#[cfg(test)]
mod tests {
    use super::{Addresses, OptionalWords, WORD_LENGTH};
    use cosmian_findex::{Address, ADDRESS_LENGTH};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    const SEED: [u8; 32] = [1u8; 32]; // arbitrary seed for the RNG

    #[test]
    fn test_ser_deser_addresses() {
        let mut rng = StdRng::from_seed(SEED);
        // Create random addresses
        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let addresses = Addresses(vec![address1, address2]);

        // Serialize the addresses
        let serialized = Addresses::serialize(addresses.clone()).expect("Serialization failed");

        // Deserialize the addresses
        let deserialized = Addresses::deserialize(serialized).expect("Deserialization failed");

        // Check that the deserialized addresses match the original addresses
        assert_eq!(addresses, deserialized, "Addresses do not match",);
    }

    #[test]
    fn test_ser_deser_optional_words() {
        let mut rng = StdRng::from_seed(SEED);

        let mut word1 = [0u8; WORD_LENGTH];
        let mut word2 = [0u8; WORD_LENGTH];
        rng.fill(&mut word1[..]);
        rng.fill(&mut word2[..]);

        let optional_words: OptionalWords<129> = OptionalWords(vec![None, Some(word1)]);

        // Serialize the optional words
        let serialized = optional_words.serialize().expect("Serialization failed");
        // Deserialize the optional words
        let deserialized = OptionalWords::deserialize(serialized).expect("Deserialization failed");

        // Check that the deserialized optional words match the original optional words
        assert_eq!(optional_words, deserialized, "Optional words do not match",);
    }
}
