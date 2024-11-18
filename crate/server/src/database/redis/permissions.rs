use async_trait::async_trait;
use cosmian_findex_structs::{Permission, Permissions};
use redis::{pipe, AsyncCommands};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};

#[async_trait]
impl PermissionsTrait for Redis {
    #[instrument(ret(Display), err, skip(self))]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let uuid = Uuid::new_v4();
        let key = user_id.as_bytes().to_vec();
        let permissions = (self.get_permissions(user_id).await).map_or_else(
            |_error| Permissions::new(uuid, Permission::Admin),
            |mut permissions| {
                permissions.grant_permission(uuid, Permission::Admin);
                permissions
            },
        );
        self.mgr
            .clone()
            .set::<_, _, ()>(key, permissions.serialize())
            .await?;

        Ok(uuid)
    }

    #[instrument(ret(Display), err, skip(self))]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let key = user_id.as_bytes().to_vec();

        let value: Option<Vec<u8>> = self.mgr.clone().get(key).await?;
        let serialized_value = value.ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission for {user_id} since unwrapping serialized value failed"
            ))
        })?;
        Permissions::deserialize(&serialized_value).map_err(FindexServerError::from)
    }

    #[instrument(ret(Display), err, skip(self))]
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let permissions = self.get_permissions(user_id).await?;
        let permission = permissions.get_permission(index_id).ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission for {user_id} on index {index_id}"
            ))
        })?;

        Ok(permission)
    }

    #[instrument(ret, err, skip(self))]
    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        let key = user_id.as_bytes().to_vec();
        let permissions = match self.get_permissions(user_id).await {
            Ok(mut permissions) => {
                permissions.grant_permission(*index_id, permission);
                permissions
            }
            Err(_) => Permissions::new(*index_id, permission),
        };

        // self.mgr
        //     .clone()
        //     .set::<_, _, ()>(key, permissions.serialize())
        //     .await
        //     .map_err(FindexServerError::from)

        let mut pipe = pipe();
        pipe.set(key, permissions.serialize());
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip(self))]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()> {
        let key = user_id.as_bytes().to_vec();
        match self.get_permissions(user_id).await {
            Ok(mut permissions) => {
                permissions.revoke_permission(index_id);
                self.mgr
                    .clone()
                    .set::<_, _, ()>(key, permissions.serialize())
                    .await?;
            }
            Err(_) => {
                trace!("Nothing to revoke since no permission found for index {index_id}");
            }
        };

        Ok(())
    }
}
