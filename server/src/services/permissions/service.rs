use std::sync::Arc;

use crate::{
    Snowflake,
    services::{permissions::Permissions, user::UserService},
};

pub struct PermissionService {
    user_service: Arc<UserService>,
}

impl PermissionService {
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    pub async fn require(
        &self,
        user: &Snowflake,
        required_perms: Permissions,
    ) -> Result<(), crate::Error> {
        let user = self
            .user_service
            .get_user(user)
            .await
            .ok_or(crate::Error::InvalidUser)?;

        if user.permissions.contains(required_perms) {
            Ok(())
        } else {
            Err(crate::Error::NoPermission(
                required_perms.difference(user.permissions),
            ))
        }
    }

    pub async fn set_user_permission(
        &self,
        target_user: Snowflake,
        permission: Permissions,
        granted: bool,
    ) -> Result<Permissions, crate::Error> {
        let user = self
            .user_service
            .get_user(&target_user)
            .await
            .ok_or(crate::Error::InvalidUser)?;

        let mut new_perms = user.permissions;
        new_perms.set(permission, granted);

        // No-op short-circuit avoids spurious broadcasts.
        if new_perms == user.permissions {
            return Ok(new_perms);
        }

        self.user_service
            .update_permissions(target_user, new_perms)
            .await?;

        Ok(new_perms)
    }
}
