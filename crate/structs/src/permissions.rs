use std::{collections::HashMap, fmt::Display, str::FromStr};

use uuid::Uuid;

use crate::{
    error::{result::StructsResult, StructsError},
    structs_bail,
};

// #[derive(Clone, Debug, ValueEnum)]
// pub enum Permission {
//     Read = 0,
//     Write = 1,
//     Admin = 2,
// }

// impl Display for Permission {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let s = match self {
//             Self::Read => "read",
//             Self::Write => "write",
//             Self::Admin => "admin",
//         };
//         write!(f, "{s}")
//     }
// }

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
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

#[derive(Debug)]
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

impl Permissions {
    pub fn new(index_id: Uuid, permission: Permission) -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(index_id, permission);
        Self { permissions }
    }

    pub fn grant_permission(&mut self, index_id: Uuid, permission: Permission) {
        self.permissions.insert(index_id, permission);
    }

    pub fn revoke_permission(&mut self, index_id: &Uuid) {
        self.permissions.remove(index_id);
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(self.permissions.len() * (PERMISSION_LENGTH + INDEX_ID_LENGTH));
        for (index_id, permission) in &self.permissions {
            bytes.extend_from_slice(&[u8::from(permission.clone())]);
            bytes.extend_from_slice(index_id.as_bytes().as_ref());
        }
        bytes
    }

    pub fn deserialize(bytes: &[u8]) -> StructsResult<Self> {
        let mut permissions = HashMap::new();
        let mut i = 0;
        while i < bytes.len() {
            let permission_u8 = bytes.get(i).ok_or_else(|| {
                StructsError::IndexingSlicing("Failed to deserialize Permission".to_owned())
            })?;
            let permission = Permission::try_from(*permission_u8)?;
            i += PERMISSION_LENGTH;
            let uuid_slice = bytes.get(i..i + INDEX_ID_LENGTH).ok_or_else(|| {
                StructsError::IndexingSlicing(
                    "Failed to extract {INDEX_ID_LENGTH} bytes from Uuid".to_owned(),
                )
            })?;
            let index_id = Uuid::from_slice(uuid_slice).map_err(|e| {
                StructsError::IndexingSlicing(format!("Failed to deserialize Uuid. Error: {e}"))
            })?;
            i += INDEX_ID_LENGTH;
            permissions.insert(index_id, permission);
        }
        Ok(Self { permissions })
    }

    pub fn get_permission(&self, index_id: &Uuid) -> Option<Permission> {
        self.permissions.get(index_id).cloned()
    }
}
