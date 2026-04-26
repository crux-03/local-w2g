use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect, handler::apply_effect},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct EditPermissionCommand {
    pub target_user: Snowflake,
    pub permission: Permissions,
    pub granted: bool,
}

#[async_trait]
impl Command for EditPermissionCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        executor_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let services = state.services();
        // Prevent self-lockout: don't let an admin drop their own MANAGE_USERS.
        if executor_id == self.target_user
            && self.permission.contains(Permissions::MANAGE_USERS)
            && !self.granted
        {
            return Err(crate::Error::CannotDemoteSelf);
        }

        let new_perms = services
            .permission()
            .set_user_permission(self.target_user, self.permission, self.granted)
            .await?;

        let target = services
            .user()
            .get_user(&self.target_user)
            .await
            .ok_or(crate::Error::InvalidUser)?;
        let executor = services
            .user()
            .get_user(&executor_id)
            .await
            .ok_or(crate::Error::InvalidUser)?;

        let target_name = target
            .display_name
            .clone()
            .unwrap_or(self.target_user.to_string());
        let executor_name = executor
            .display_name
            .clone()
            .unwrap_or(executor_id.to_string());

        permission_update_system_log(
            &state,
            executor_name,
            target_name,
            self.permission,
            self.granted,
        )
        .await?;

        Ok(ServerMessage::PermissionUpdate {
            user_id: self.target_user,
            permissions: new_perms,
        }
        .into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_USERS)
    }
}

async fn permission_update_system_log(
    state: &Arc<AppState>,
    executor_name: String,
    target_name: String,
    permission: Permissions,
    granted: bool,
) -> Result<(), crate::Error> {
    let perm_name = permission
        .iter_names()
        .map(|(name, _)| name)
        .next()
        .unwrap_or("UNKNOWN");

    let content = if granted {
        format!("{executor_name} granted {target_name} the {perm_name} permission")
    } else {
        format!("{executor_name} revoked the {perm_name} permission from {target_name}")
    };

    let log = state.services().message().system_log(content).await;
    apply_effect(
        state,
        Effect::Global(ServerMessage::MessageCreated { entry: log }),
    )
    .await?;
    Ok(())
}
