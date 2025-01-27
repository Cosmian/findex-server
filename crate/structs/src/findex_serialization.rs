/// this module contains serialization and deserialization functions needed for the Findex server
use crate::StructsError;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{Address, ADDRESS_LENGTH};

pub type SerializationResult<R> = Result<R, StructsError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addresses(Vec<Address<ADDRESS_LENGTH>>);

// Serialization functions
impl Addresses {
    #[must_use]
    pub const fn new(addresses: Vec<Address<ADDRESS_LENGTH>>) -> Self {
        Self(addresses)
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<Address<ADDRESS_LENGTH>> {
        self.0
    }
    /// Serializes the `Addresses` instance into a vector of bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len().try_into()?)
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        for adr in &self.0 {
            ser.write_array(adr.as_ref())
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into an `Addresses` instance.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);
        for _ in 0..length {
            let key: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            items.push(key);
        }
        Ok(Self(items))
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
    word: Option<&[u8; WORD_LENGTH]>,
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
            "Invalid value for serialized option flag".to_owned(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalWords<const WORD_LENGTH: usize>(Vec<Option<[u8; WORD_LENGTH]>>);

impl<const WORD_LENGTH: usize> From<OptionalWords<WORD_LENGTH>> for Vec<Option<[u8; WORD_LENGTH]>> {
    fn from(words: OptionalWords<WORD_LENGTH>) -> Self {
        words.0
    }
}

impl<const WORD_LENGTH: usize> OptionalWords<WORD_LENGTH> {
    /// Creates a new `OptionalWords` instance.
    #[must_use]
    pub const fn new(words: Vec<Option<[u8; WORD_LENGTH]>>) -> Self {
        Self(words)
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<Option<[u8; WORD_LENGTH]>> {
        self.0
    }

    /// Serializes the `OptionalWords` instance into a vector of bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len().try_into().map_err(|e| {
            StructsError::SerializationError(format!(
                "Length conversion failed. Original error : {e}"
            ))
        })?)?;
        for word in &self.0 {
            ser_optional_word(&mut ser, word.as_ref())
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into an `OptionalWords` instance.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);

        for _ in 0..length {
            let word = deser_optional_word::<WORD_LENGTH>(&mut de)?;
            items.push(word);
        }
        Ok(Self(items))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard<const WORD_LENGTH: usize>(Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>);

impl<const WORD_LENGTH: usize> Guard<WORD_LENGTH> {
    /// Creates a new `Guard` instance.
    #[must_use]
    pub const fn new(address: Address<ADDRESS_LENGTH>, word: Option<[u8; WORD_LENGTH]>) -> Self {
        Self(address, word)
    }

    #[must_use]
    pub const fn into_inner(self) -> (Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>) {
        (self.0, self.1)
    }

    #[must_use]
    pub const fn possible_sizes() -> (usize, usize) {
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
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
        let flag = <usize>::try_from(de.read_leb128_u64()?)?;
        let word = if flag == 1 {
            Some(de.read_array()?)
        } else if flag == 0 {
            None
        } else {
            return Err(StructsError::DeserializationError(
                "Invalid value for serialized option flag".to_owned(),
            ));
        };
        Ok(Self(address, word))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tasks<const WORD_LENGTH: usize>(Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>);

impl<const WORD_LENGTH: usize> Tasks<WORD_LENGTH> {
    /// Creates a new `Tasks` instance.
    #[must_use]
    pub const fn new(tasks: Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>) -> Self {
        Self(tasks)
    }

    #[must_use]
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
        ser.write_leb128_u64(self.0.len().try_into().map_err(|e| {
            StructsError::SerializationError(format!(
                "Length conversion failed. Original error : {e}"
            ))
        })?)?;
        for (address, word) in &self.0 {
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
    ///    - The address iz deserialized from a byte array.
    ///    - The word is deserialized from a byte array.
    ///
    /// # Errors
    ///
    /// Returns a `DeserializationError` if any step of the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);
        for _ in 0..length {
            let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            let word: [u8; WORD_LENGTH] = de.read_array()?;
            items.push((address, word));
        }
        Ok(Self(items))
    }
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use crate::findex_serialization::Guard;

    use super::{Addresses, OptionalWords, Tasks};
    use cosmian_findex::{Address, ADDRESS_LENGTH, WORD_LENGTH};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    const SEED: [u8; 32] = [1_u8; 32]; // arbitrary seed for the RNG

    #[test]
    fn test_ser_deser_addresses() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let addresses = Addresses(vec![address1, address2]);

        let serialized = addresses.serialize().expect("Serialization failed");
        let deserialized = Addresses::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(addresses, deserialized, "Addresses do not match",);
    }

    #[test]
    fn test_ser_deser_optional_words() {
        let mut rng = StdRng::from_seed(SEED);

        let mut word1 = [0_u8; WORD_LENGTH];
        let mut word2 = [0_u8; WORD_LENGTH];
        rng.fill(&mut word1[..]);
        rng.fill(&mut word2[..]);

        let optional_words: OptionalWords<{ WORD_LENGTH }> = OptionalWords(vec![None, Some(word1)]);

        let serialized = optional_words.serialize().expect("Serialization failed");
        let deserialized = OptionalWords::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(optional_words, deserialized, "Optional words do not match",);
    }

    #[test]
    fn test_ser_deser_guard() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let mut word = [0_u8; WORD_LENGTH];
        rng.fill(&mut word[..]);

        let guard_some: Guard<WORD_LENGTH> = Guard(address1, Some(word));
        let serialized_some = guard_some.serialize().expect("Serialization failed");
        let deserialized_some =
            Guard::deserialize(&serialized_some).expect("Deserialization failed");

        assert_eq!(
            guard_some, deserialized_some,
            "Guard with Some(word) does not match"
        );

        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
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
    fn test_ser_deser_tasks() {
        let mut rng = StdRng::from_seed(SEED);

        let address1: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let address2: Address<ADDRESS_LENGTH> = rng.gen::<u128>().to_be_bytes().into();
        let mut word1 = [0_u8; WORD_LENGTH];
        let mut word2 = [0_u8; WORD_LENGTH];
        rng.fill(&mut word1[..]);
        rng.fill(&mut word2[..]);

        let tasks = Tasks(vec![(address1, word1), (address2, word2)]);

        let serialized = tasks.serialize().expect("Serialization failed");
        let deserialized = Tasks::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(tasks, deserialized, "Tasks do not match");
    }
}
