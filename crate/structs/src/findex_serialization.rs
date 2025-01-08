/// this module contains serialization and deserialization functions needed for the Findex server
use crate::StructsError;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{ADDRESS_LENGTH, Address};

pub type SerializationResult<R> = Result<R, StructsError>;

#[derive(Debug, Clone, PartialEq)]
pub struct Addresses(Vec<Address<ADDRESS_LENGTH>>);

// Serialization functions
impl Addresses {
    pub fn new(addresses: Vec<Address<ADDRESS_LENGTH>>) -> Self {
        Addresses(addresses)
    }

    pub fn into_inner(self) -> Vec<Address<ADDRESS_LENGTH>> {
        self.0
    }
    /// Serializes the `Addresses` instance into a vector of bytes.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len() as u64)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for adr in self.0.iter() {
            ser.write_array(adr.as_ref())
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into an `Addresses` instance.
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
fn ser_optional_word<const WORD_LENGTH: usize>(
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

/// Deserializes an optional word from a deserializer.
///
/// The deserialization protocol is as follows:
/// 1. A flag is read from the deserializer.
/// 2. If the flag is `0`, `None` is returned.
/// 3. If the flag is `1`, a word is read from the deserializer and returned as `Some`.
///
/// # Errors
///
/// Returns a `DeserializationError` if any step of the deserialization process fails.
fn deser_optional_word<const WORD_LENGTH: usize>(
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
    /// Creates a new `OptionalWords` instance.
    pub fn new(words: Vec<Option<[u8; WORD_LENGTH]>>) -> Self {
        OptionalWords(words)
    }

    // Public method to access the inner vector
    pub fn into_inner(self) -> Vec<Option<[u8; WORD_LENGTH]>> {
        self.0
    }

    /// Serializes the `OptionalWords` instance into a vector of bytes.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len() as u64)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for word in self.0.iter() {
            ser_optional_word(&mut ser, word)
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into an `OptionalWords` instance.
    pub fn deserialize(data: Vec<u8>) -> SerializationResult<Self> {
        let mut de = Deserializer::new(&data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);

        for _ in 0..length {
            let word = deser_optional_word::<WORD_LENGTH>(&mut de)?;
            items.push(word);
        }
        Ok(OptionalWords(items))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Guard<const WORD_LENGTH: usize>(Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>);

impl<const WORD_LENGTH: usize> Guard<WORD_LENGTH> {
    /// Creates a new `Guard` instance.
    pub fn new(address: Address<ADDRESS_LENGTH>, word: Option<[u8; WORD_LENGTH]>) -> Self {
        Guard(address, word)
    }

    pub fn into_inner(self) -> (Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>) {
        (self.0, self.1)
    }

    pub fn possible_sizes() -> (usize, usize) {
        (ADDRESS_LENGTH + 1, ADDRESS_LENGTH + 1 + WORD_LENGTH)
    }

    /// Serializes the `Guard` instance into a vector of bytes.
    ///
    /// The serialization protocol is as follows:
    /// 1. The address is serialized as a byte array.
    /// 2. The optional word is serialized as follows:
    ///    - If the word is `Some`, a `1` is written followed by the byte array.
    ///    - If the word is `None`, a `0` is written.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if any step of the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(ADDRESS_LENGTH + WORD_LENGTH);
        ser.write_array(self.0.as_ref())
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        match &self.1 {
            Some(word) => {
                ser.write_leb128_u64(1)?;
                ser.write_array(word)
            }
            None => ser.write_leb128_u64(0),
        }
        .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        let our_result = ser.finalize().to_vec();
        Ok(our_result)
    }

    /// Deserializes a vector of bytes into a `Guard` instance.
    ///
    /// The deserialization protocol is as follows:
    /// 1. The address is deserialized from a byte array.
    /// 2. The optional word is deserialized as follows:
    ///    - If the next byte is `1`, the word is deserialized from the following byte array.
    ///    - If the next byte is `0`, the word is set to `None`.
    ///
    /// # Errors
    ///
    /// Returns a `DeserializationError` if any step of the deserialization process fails.
    pub fn deserialize(data: Vec<u8>) -> SerializationResult<Self> {
        let mut de = Deserializer::new(&data);
        let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
        let flag = <usize>::try_from(de.read_leb128_u64()?)?;
        let word = if flag == 1 {
            Some(de.read_array()?)
        } else if flag == 0 {
            None
        } else {
            return Err(StructsError::DeserializationError(
                "Invalid value for serialized option flag".to_string(),
            ));
        };
        Ok(Guard(address, word))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tasks<const WORD_LENGTH: usize>(Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>);

impl<const WORD_LENGTH: usize> Tasks<WORD_LENGTH> {
    /// Creates a new `Tasks` instance.
    pub fn new(tasks: Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>) -> Self {
        Tasks(tasks)
    }

    pub fn into_inner(self) -> Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])> {
        self.0
    }

    /// Serializes the `Tasks` instance into a vector of bytes.
    ///
    /// The serialization protocol is as follows:
    /// 1. The length of the vector is serialized as a LEB128-encoded u64.
    /// 2. Each element in the vector is serialized as follows:
    ///    - The address is serialized as a byte array.
    ///    - The word is serialized as a byte array.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if any step of the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len() as u64)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for (address, word) in self.0.iter() {
            {
                ser.write_array(address.as_ref())?;
                ser.write_array(word)
            }
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into a `Tasks` instance.
    ///
    /// The deserialization protocol is as follows:
    /// 1. The length of the vector is deserialized from a LEB128-encoded u64.
    /// 2. Each element in the vector is deserialized as follows:
    ///    - The address is deserialized from a byte array.
    ///    - The word is deserialized from a byte array.
    ///
    /// # Errors
    ///
    /// Returns a `DeserializationError` if any step of the deserialization process fails.
    pub fn deserialize(data: Vec<u8>) -> SerializationResult<Self> {
        let mut de = Deserializer::new(&data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);
        for _ in 0..length {
            let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            let word: [u8; WORD_LENGTH] = de.read_array()?;
            items.push((address, word));
        }
        Ok(Tasks(items))
    }
}

#[cfg(test)]
mod tests {
    use crate::WORD_LENGTH;
    use crate::findex_serialization::Guard;

    use super::{Addresses, OptionalWords, Tasks};
    use cosmian_findex::{ADDRESS_LENGTH, Address};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    const SEED: [u8; 32] = [1u8; 32]; // arbitrary seed for the RNG

    #[test]
    fn test_ser_deser_addresses() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let addresses = Addresses(vec![address1, address2]);

        let serialized = addresses.serialize().expect("Serialization failed");
        let deserialized = Addresses::deserialize(serialized).expect("Deserialization failed");

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

        let serialized = optional_words.serialize().expect("Serialization failed");
        let deserialized = OptionalWords::deserialize(serialized).expect("Deserialization failed");

        assert_eq!(optional_words, deserialized, "Optional words do not match",);
    }

    #[test]
    fn test_ser_deser_guard() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let mut word = [0u8; WORD_LENGTH];
        rng.fill(&mut word[..]);

        let guard_some: Guard<WORD_LENGTH> = Guard(address1, Some(word));
        let serialized_some = guard_some.serialize().expect("Serialization failed");
        let deserialized_some =
            Guard::deserialize(serialized_some).expect("Deserialization failed");

        assert_eq!(
            guard_some, deserialized_some,
            "Guard with Some(word) does not match"
        );

        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let guard_none: Guard<WORD_LENGTH> = Guard(address2, None);

        let serialized_none = guard_none.serialize().expect("Serialization failed");
        let deserialized_none =
            Guard::deserialize(serialized_none).expect("Deserialization failed");

        assert_eq!(
            guard_none, deserialized_none,
            "Guard with None does not match"
        );
    }

    #[test]
    fn test_ser_deser_tasks() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let mut word1 = [0u8; WORD_LENGTH];
        let mut word2 = [0u8; WORD_LENGTH];
        rng.fill(&mut word1[..]);
        rng.fill(&mut word2[..]);

        let tasks = Tasks(vec![(address1, word1), (address2, word2)]);

        let serialized = tasks.serialize().expect("Serialization failed");
        let deserialized = Tasks::deserialize(serialized).expect("Deserialization failed");

        assert_eq!(tasks, deserialized, "Tasks do not match");
    }
}
