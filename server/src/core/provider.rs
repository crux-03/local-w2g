use std::sync::Arc;

use crate::services::{
    message::MessageService, permissions::PermissionService, playback::PlaybackService,
    snowflake::SnowflakeService, state::StateService, user::UserService, videos::VideoService,
};

pub struct ServiceProvider {
    snowflake: Arc<SnowflakeService>,
    users: Arc<UserService>,
    permissions: Arc<PermissionService>,
    messages: Arc<MessageService>,
    playback: Arc<PlaybackService>,
    video: Arc<VideoService>,
    state: Arc<StateService>,
}

impl ServiceProvider {
    pub async fn new() -> Result<Self, crate::Error> {
        let snowflake_service = Arc::new(SnowflakeService::new(1));
        let user_service = Arc::new(UserService::new(Arc::clone(&snowflake_service)));
        let permission_service = Arc::new(PermissionService::new(Arc::clone(&user_service)));
        let message_service = Arc::new(MessageService::new(100, Arc::clone(&snowflake_service)));
        let playback_service = Arc::new(PlaybackService::new(Arc::clone(&snowflake_service)));
        let video_service =
            Arc::new(VideoService::new("../videos", Arc::clone(&snowflake_service)).await?);
        let state_service = Arc::new(StateService::new(
            Arc::clone(&user_service),
            Arc::clone(&video_service),
        ));

        Ok(Self {
            snowflake: snowflake_service,
            users: user_service,
            permissions: permission_service,
            messages: message_service,
            playback: playback_service,
            video: video_service,
            state: state_service,
        })
    }

    pub fn snowflake(&self) -> &Arc<SnowflakeService> {
        &self.snowflake
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

    pub fn video(&self) -> &Arc<VideoService> {
        &self.video
    }

    pub fn state(&self) -> &Arc<StateService> {
        &self.state
    }
}
