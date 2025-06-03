use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, Permission, Permissions};
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::database::{
    DatabaseError, database_traits::PermissionsTrait, findex_database::DatabaseResult,
};

const PERMISSIONS_PREFIX: &str = "permissions";

async fn hset_redis_permission(
    manager: &ConnectionManager,
    user_id: &str,
    index_id: &Uuid,
    permission: Permission,
) -> Result<(), RedisError> {
    let user_redis_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

    manager
        .clone()
        .hset::<_, _, u8, _>(&user_redis_key, index_id.to_string(), u8::from(permission))
        .await
}

#[async_trait]
impl PermissionsTrait for Redis<CUSTOM_WORD_LENGTH> {
    /// Creates a new index ID and sets admin privileges.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> DatabaseResult<Uuid> {
        let index_id = Uuid::new_v4();
        hset_redis_permission(&self.manager, user_id, &index_id, Permission::Admin).await?;
        trace!("New index with id {index_id} created for user  {user_id}");
        Ok(index_id)
    }

    /// Sets a permission to a user for a specific index.
    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> DatabaseResult<()> {
        hset_redis_permission(&self.manager, user_id, index_id, permission).await?;
        trace!("Set {permission:?} permission to {user_id} for index {index_id}");
        Ok(())
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> DatabaseResult<Permissions> {
        let user_redis_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        let permissions: Permissions = self
            .manager
            .clone()
            .hgetall::<_, Vec<(String, u8)>>(user_redis_key)
            .await?
            .into_iter()
            .map(|(index_str, perm)| {
                Ok((
                    Uuid::parse_str(&index_str).map_err(|e| {
                        DatabaseError::InvalidDatabaseResponse(format!("Invalid index ID. {e}"))
                    })?,
                    Permission::try_from(perm).map_err(|e| {
                        DatabaseError::InvalidDatabaseResponse(format!("Invalid index ID. {e}"))
                    })?,
                ))
            })
            .collect::<DatabaseResult<_>>()?;

        trace!("permissions for user {user_id}: {permissions:?}");
        Ok(permissions)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<Permission> {
        let user_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        let permission = self
            .manager
            .clone()
            .hget::<_, _, Option<u8>>(&user_key, index_id.to_string())
            .await?
            .ok_or_else(|| {
                DatabaseError::InvalidDatabaseResponse(
                    "No permission found for index {index_id}".to_owned(),
                )
            })
            .and_then(|p| {
                Permission::try_from(p).map_err(|e| {
                    DatabaseError::InvalidDatabaseResponse(format!(
                        "An invalid permission value was returned by the database. {e}"
                    ))
                })
            })?;

        trace!("Permissions for user {user_id}: {permission:?}");
        Ok(permission)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<()> {
        let user_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        // never type fallbacks will be deprecated in future Rust releases, hence this explicit typing
        let _: () = self
            .manager
            .clone()
            .hdel(&user_key, index_id.to_string())
            .await
            .map_err(DatabaseError::from)?;

        trace!("Revoked permission for {user_id} on index {index_id}");
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::get_unwrap,
    clippy::too_many_lines
)]
mod tests {
    use std::env;

    use tokio;
    use tracing::debug;

    use super::*;
    use crate::{
        config::DatabaseType,
        database::{
            database_traits::InstantiationTrait,
            test_utils::permission_tests::{
                concurrent_create_index_id, concurrent_set_revoke_permissions, create_index_id,
                nonexistent_user_and_permission, revoke_permission, set_and_revoke_permissions,
            },
        },
        generate_permission_tests,
    };

    async fn setup_test_db() -> Redis<CUSTOM_WORD_LENGTH> {
        let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_owned());
        Redis::instantiate(DatabaseType::Redis, url.as_str(), false)
            .await
            .expect("Test failed to instantiate Redis")
    }

    generate_permission_tests! {
        setup_test_db().await;
        create_index_id,
        set_and_revoke_permissions,
        revoke_permission,
        nonexistent_user_and_permission,
        concurrent_set_revoke_permissions,
        concurrent_create_index_id
    }
}
