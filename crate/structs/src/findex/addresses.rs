use crate::StructsError;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{ADDRESS_LENGTH, Address};

use super::SerializationResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addresses(pub Vec<Address<ADDRESS_LENGTH>>);

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
            let seed: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            items.push(seed);
        }
        Ok(Self(items))
    }
}
