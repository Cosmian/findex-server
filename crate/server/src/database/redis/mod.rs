use argon2::Argon2;
use async_trait::async_trait;
use cloudproof::reexport::crypto_core::{reexport::rand_core::SeedableRng, FixedSizeCBytes};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use cloudproof::reexport::crypto_core::{kdf256, Aes256Gcm, CsRng, Instantiable, SymmetricKey};
use cloudproof_findex::{
    implementations::redis::{FindexRedis, FindexRedisError, RemovedLocationsFinder},
    parameters::MASTER_KEY_LENGTH,
    Label, Location,
};
use redis::aio::ConnectionManager;

use crate::{error::FindexServerError, result::FResult, secret::Secret};

use super::Database;

pub(crate) const REDIS_WITH_FINDEX_MASTER_KEY_LENGTH: usize = 32;
pub(crate) const REDIS_WITH_FINDEX_MASTER_KEY_DERIVATION_SALT: &[u8; 16] = b"rediswithfindex_";
pub(crate) const REDIS_WITH_FINDEX_MASTER_FINDEX_KEY_DERIVATION_SALT: &[u8; 6] = b"findex";
pub(crate) const REDIS_WITH_FINDEX_MASTER_DB_KEY_DERIVATION_SALT: &[u8; 2] = b"db";

pub(crate) const DB_KEY_LENGTH: usize = 32;

#[allow(dead_code)]
pub(crate) struct ObjectsDB {
    mgr: ConnectionManager,
    dem: Aes256Gcm,
    rng: Mutex<CsRng>,
}

impl ObjectsDB {
    pub(crate) fn new(mgr: ConnectionManager, db_key: &SymmetricKey<DB_KEY_LENGTH>) -> Self {
        Self {
            mgr,
            dem: Aes256Gcm::new(db_key),
            rng: Mutex::new(CsRng::from_entropy()),
        }
    }
}
#[async_trait]
impl RemovedLocationsFinder for ObjectsDB {
    async fn find_removed_locations(
        &self,
        _locations: HashSet<Location>,
    ) -> Result<HashSet<Location>, FindexRedisError> {
        // Objects and permissions are never removed from the DB
        Ok(HashSet::new())
    }
}

#[allow(dead_code)]
pub(crate) struct RedisWithFindex {
    objects_db: Arc<ObjectsDB>,
    findex: Arc<FindexRedis>,
    findex_key: SymmetricKey<MASTER_KEY_LENGTH>,
    label: Label,
}

impl RedisWithFindex {
    pub(crate) async fn instantiate(
        redis_url: &str,
        master_key: Secret<REDIS_WITH_FINDEX_MASTER_KEY_LENGTH>,
        label: &[u8],
    ) -> FResult<Self> {
        // derive a Findex Key
        let mut findex_key = SymmetricKey::<MASTER_KEY_LENGTH>::default();
        kdf256!(
            &mut findex_key,
            REDIS_WITH_FINDEX_MASTER_FINDEX_KEY_DERIVATION_SALT,
            &*master_key
        );
        // derive a DB Key
        let mut db_key = SymmetricKey::<DB_KEY_LENGTH>::default();
        kdf256!(
            &mut db_key,
            REDIS_WITH_FINDEX_MASTER_DB_KEY_DERIVATION_SALT,
            &*master_key
        );

        let client = redis::Client::open(redis_url)?;
        let mgr = ConnectionManager::new(client).await?;
        let objects_db = Arc::new(ObjectsDB::new(mgr.clone(), &db_key));
        let findex =
            Arc::new(FindexRedis::connect_with_manager(mgr.clone(), objects_db.clone()).await?);
        Ok(Self {
            objects_db,
            findex,
            findex_key,
            label: Label::from(label),
        })
    }

    pub(crate) fn master_key_from_password(
        master_password: &str,
    ) -> FResult<SymmetricKey<REDIS_WITH_FINDEX_MASTER_KEY_LENGTH>> {
        let output_key_material = derive_key_from_password::<REDIS_WITH_FINDEX_MASTER_KEY_LENGTH>(
            REDIS_WITH_FINDEX_MASTER_KEY_DERIVATION_SALT,
            master_password.as_bytes(),
        )?;

        let master_secret_key: SymmetricKey<REDIS_WITH_FINDEX_MASTER_KEY_LENGTH> =
            SymmetricKey::try_from_slice(&output_key_material)?;

        Ok(master_secret_key)
    }
}

pub(crate) fn derive_key_from_password<const LENGTH: usize>(
    salt: &[u8; 16],
    password: &[u8],
) -> Result<Secret<LENGTH>, FindexServerError> {
    let mut output_key_material = Secret::<LENGTH>::new();

    Argon2::default()
        .hash_password_into(password, salt, output_key_material.as_mut())
        .map_err(|e| FindexServerError::CryptographicError(e.to_string()))?;

    Ok(output_key_material)
}

#[async_trait(?Send)]
impl Database for RedisWithFindex {
    async fn create(&self) -> FResult<()> {
        Ok(())
    }
}
