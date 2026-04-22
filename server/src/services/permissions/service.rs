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
}
