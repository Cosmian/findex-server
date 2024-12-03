use std::{fmt::Display, ops::Deref};

use cloudproof_findex::reexport::cosmian_crypto_core::bytes_ser_de::{
    self, to_leb128_len, Serializable,
};
use uuid::Uuid;

use crate::{encrypted_entries::UUID_LENGTH, StructsError};

#[derive(Debug)]
pub struct Uuids {
    pub uuids: Vec<Uuid>,
}

impl Deref for Uuids {
    type Target = Vec<Uuid>;

    fn deref(&self) -> &Self::Target {
        &self.uuids
    }
}

impl Display for Uuids {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for uuid in &self.uuids {
            writeln!(f, "UUID: {uuid}")?;
        }
        Ok(())
    }
}

impl From<Vec<Uuid>> for Uuids {
    fn from(uuids: Vec<Uuid>) -> Self {
        Self { uuids }
    }
}

impl Serializable for Uuids {
    type Error = StructsError;

    fn length(&self) -> usize {
        let uuids_len = self.uuids.len() * UUID_LENGTH;
        to_leb128_len(uuids_len) + uuids_len
    }

    /// Serialize the Uuids struct
    ///
    /// | Field       | Type   | Description                          |
    /// |-------------|--------|--------------------------------------|
    /// | uuids       | Vec<Uuid> | A vector of UUIDs to be serialized |
    ///
    /// The serialization format is as follows:
    /// 1. The number of UUIDs (encoded as LEB128).
    /// 2. The UUIDs themselves, each serialized as a 16-byte array.
    fn write(&self, ser: &mut bytes_ser_de::Serializer) -> Result<usize, Self::Error> {
        let mut n = ser.write_leb128_u64(u64::try_from(self.uuids.len())?)?;
        for uuid in &self.uuids {
            n += ser.write_array(uuid.as_bytes())?;
        }
        Ok(n)
    }

    fn read(de: &mut bytes_ser_de::Deserializer) -> Result<Self, Self::Error> {
        let nb = de.read_leb128_u64()?;
        let mut uuids = Vec::with_capacity(usize::try_from(nb)? * UUID_LENGTH);
        for _ in 0..nb {
            let uuid = de.read_array::<UUID_LENGTH>()?;
            uuids.push(Uuid::from_slice(&uuid)?);
        }
        Ok(Self { uuids })
    }
}

#[cfg(test)]
mod tests {
    use cloudproof_findex::reexport::cosmian_crypto_core::bytes_ser_de::Serializable;
    use uuid::Uuid;

    use super::Uuids;
    use crate::error::result::StructsResult;

    #[test]
    #[allow(clippy::panic_in_result_fn)]
    fn test_uuids() -> StructsResult<()> {
        let uuids = Uuids {
            uuids: vec![
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
            ],
        };
        let bytes = uuids.serialize()?;
        let deserialized_uuids = Uuids::deserialize(bytes.as_ref())?;
        assert_eq!(uuids.uuids, deserialized_uuids.uuids);
        Ok(())
    }
}
