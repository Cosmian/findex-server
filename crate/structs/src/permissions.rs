use std::{collections::HashMap, fmt::Display, str::FromStr};

use cosmian_crypto_core::bytes_ser_de::{self, Serializable, to_leb128_len};
use tracing::debug;
use uuid::Uuid;

use crate::{
    error::{StructsError, result::StructsResult},
    structs_bail,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    Read = 0,
    Write = 1,
    Admin = 2,
}

#[allow(clippy::as_conversions)]
impl From<Permission> for u8 {
    fn from(table: Permission) -> Self {
        table as Self
    }
}

impl TryFrom<u8> for Permission {
    type Error = StructsError;

    fn try_from(value: u8) -> StructsResult<Self> {
        match value {
            0 => Ok(Self::Read),
            1 => Ok(Self::Write),
            2 => Ok(Self::Admin),
            _ => structs_bail!("Invalid permission: {}", value),
        }
    }
}

impl FromStr for Permission {
    type Err = StructsError;

    fn from_str(s: &str) -> StructsResult<Self> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            _ => structs_bail!("Invalid permission: {}", s),
        }
    }
}

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        };
        write!(f, "{s}")
    }
}

const PERMISSION_LENGTH: usize = 1;
const INDEX_ID_LENGTH: usize = 16;

/// Map of index id <--> permission for a user
/// The key is the index id and the value is the permission
/// Each entry has a length of 17 bytes
///
/// | Index ID (UUID) | Permission |
/// |-----------------|------------|
/// | 16 bytes        | 1 byte     |
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    pub permissions: HashMap<Uuid, Permission>,
}

impl Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index_id, permission) in &self.permissions {
            writeln!(f, "Index ID: {index_id}, Permission: {permission}")?;
        }
        Ok(())
    }
}

impl FromIterator<(Uuid, Permission)> for Permissions {
    fn from_iter<I: IntoIterator<Item = (Uuid, Permission)>>(iter: I) -> Self {
        let mut permissions = HashMap::new();
        for (uuid, permission) in iter {
            permissions.insert(uuid, permission);
        }
        Self { permissions }
    }
}

impl Serializable for Permissions {
    type Error = StructsError;

    fn length(&self) -> usize {
        let permissions_len = self.permissions.len() * (PERMISSION_LENGTH + INDEX_ID_LENGTH);
        to_leb128_len(permissions_len) + permissions_len
    }

    /// Serialize the permissions
    ///
    /// | Field        | Type   | Length (bytes) | Description                |
    /// |--------------|--------|----------------|----------------------------|
    /// | Permissions  | u64    | variable       | Number of permissions      |
    /// | Index ID     | [u8; 16] | 16           | UUID of the index          |
    /// | Permission   | u8     | 1              | Permission value (0, 1, 2) |
    fn write(&self, ser: &mut bytes_ser_de::Serializer) -> Result<usize, Self::Error> {
        let mut n = ser.write_leb128_u64(u64::try_from(self.permissions.len())?)?;
        for (index_id, permission) in &self.permissions {
            n += ser.write_leb128_u64(u64::from(u8::from(*permission)))?;
            n += ser.write_array(index_id.as_bytes())?;
        }
        Ok(n)
    }

    fn read(de: &mut bytes_ser_de::Deserializer) -> Result<Self, Self::Error> {
        let nb = de.read_leb128_u64()?;
        let length = usize::try_from(nb)? * PERMISSION_LENGTH + INDEX_ID_LENGTH;
        if length > 1_000_000 {
            debug!("Permissions: read: allocating {length}");
        }

        let mut permissions = HashMap::with_capacity(length);
        for _ in 0..nb {
            let permission_u8 = u8::try_from(de.read_leb128_u64()?)?;
            let permission = Permission::try_from(permission_u8)?;
            let uuid = de.read_array::<INDEX_ID_LENGTH>()?;
            let index_id = Uuid::from_slice(&uuid)?;
            permissions.insert(index_id, permission);
        }
        Ok(Self { permissions })
    }
}

impl Permissions {
    #[must_use]
    pub fn new(index_id: Uuid, permission: Permission) -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(index_id, permission);
        Self { permissions }
    }

    pub fn set_permission(&mut self, index_id: Uuid, permission: Permission) {
        self.permissions.insert(index_id, permission);
    }

    pub fn revoke_permission(&mut self, index_id: &Uuid) {
        self.permissions.remove(index_id);
    }

    #[must_use]
    pub fn get_permission(&self, index_id: &Uuid) -> Option<&Permission> {
        self.permissions.get(index_id)
    }

    #[must_use]
    pub fn min(&self, other_permissions: &Self) -> Self {
        let mut permissions = HashMap::with_capacity(self.permissions.len());
        for (index_id, permission) in &self.permissions {
            if let Some(other_permission) = other_permissions.permissions.get(index_id) {
                permissions.insert(*index_id, std::cmp::min(*permission, *other_permission));
            }
        }
        Self { permissions }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions_ser_de() {
        let permissions = Permissions {
            permissions: vec![
                (Uuid::new_v4(), Permission::Read),
                (Uuid::new_v4(), Permission::Write),
                (Uuid::new_v4(), Permission::Admin),
            ]
            .into_iter()
            .collect(),
        };

        let serialized_permissions = permissions.serialize().unwrap();
        let deserialized_permissions = Permissions::deserialize(&serialized_permissions).unwrap();
        assert_eq!(permissions, deserialized_permissions);
    }

    #[test]
    fn test_permissions_min() {
        let permissions = Permissions {
            permissions: vec![
                (
                    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                    Permission::Read,
                ),
                (
                    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
                    Permission::Write,
                ),
                (
                    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
                    Permission::Admin,
                ),
            ]
            .into_iter()
            .collect(),
        };

        let other_permissions = Permissions {
            permissions: vec![
                (
                    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                    Permission::Read,
                ),
                (
                    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
                    Permission::Write,
                ),
                (
                    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
                    Permission::Admin,
                ),
            ]
            .into_iter()
            .collect(),
        };

        let min_permissions = permissions.min(&other_permissions);
        assert_eq!(permissions, min_permissions);

        let other_permissions = Permissions {
            permissions: vec![
                (
                    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                    Permission::Admin,
                ),
                (
                    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
                    Permission::Admin,
                ),
            ]
            .into_iter()
            .collect(),
        };
        let min_permissions = permissions.min(&other_permissions);
        assert_ne!(permissions, min_permissions);
        assert_eq!(min_permissions.permissions.len(), 2);
        assert_eq!(
            min_permissions
                .permissions
                .get(&Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()),
            Some(&Permission::Read)
        );
        assert_eq!(
            min_permissions
                .permissions
                .get(&Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
            Some(&Permission::Write)
        );
    }
}
