use std::{fmt::Display, ops::Deref};

use uuid::Uuid;

use crate::{encrypted_entries::UUID_LENGTH, error::result::StructsResult, StructsError};

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

impl Uuids {
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for uuid in &self.uuids {
            bytes.extend_from_slice(uuid.as_bytes().as_ref());
        }
        bytes
    }

    pub fn deserialize(bytes: &[u8]) -> StructsResult<Self> {
        let mut uuids = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let uuid_slice = bytes.get(i..i + UUID_LENGTH).ok_or_else(|| {
                StructsError::IndexingSlicing("UUID indexing slicing failed".to_string())
            })?;
            let uuid = Uuid::from_slice(uuid_slice)?;
            i += UUID_LENGTH;
            uuids.push(uuid);
        }
        Ok(Self { uuids })
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::Uuids;

    #[test]
    fn test_uuids() {
        let uuids = Uuids {
            uuids: vec![
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
            ],
        };
        let bytes = uuids.serialize();
        let deserialized_uuids = Uuids::deserialize(&bytes).unwrap();
        assert_eq!(uuids.uuids, deserialized_uuids.uuids);
    }
}
