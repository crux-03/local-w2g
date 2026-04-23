use std::sync::Arc;

use crate::services::{
    message::MessageService, permissions::PermissionService, playback::PlaybackService,
    snowflake::SnowflakeService, user::UserService,
};

pub struct ServiceProvider {
    users: Arc<UserService>,
    permissions: Arc<PermissionService>,
    messages: Arc<MessageService>,
    playback: Arc<PlaybackService>,
}

impl ServiceProvider {
    pub fn new() -> Self {
        let snowflake_service = Arc::new(SnowflakeService::new(1));
        let user_service = Arc::new(UserService::new(Arc::clone(&snowflake_service)));
        let permission_service = Arc::new(PermissionService::new(Arc::clone(&user_service)));
        let message_service = Arc::new(MessageService::new(100, Arc::clone(&snowflake_service)));
        let playback_service = Arc::new(PlaybackService::new(Arc::clone(&snowflake_service)));

        Self {
            users: user_service,
            permissions: permission_service,
            messages: message_service,
            playback: playback_service,
        }
    }

    pub fn user(&self) -> &Arc<UserService> {
        &self.users
    }

    pub fn permission(&self) -> &Arc<PermissionService> {
        &self.permissions
    }

    pub fn message(&self) -> &Arc<MessageService> {
        &self.messages
    }

    pub fn playback(&self) -> &Arc<PlaybackService> {
        &self.playback
    }
}
