pub mod download;
pub mod handler;
pub mod messages;
pub mod misc;
pub mod playback;
pub mod playlist;
pub mod resync;
pub mod state;
pub mod user;

use std::sync::Arc;

use crate::Snowflake;
use async_trait::async_trait;

use crate::{core::AppState, services::permissions::Permissions, websocket::ServerMessage};

#[async_trait]
pub trait Command: Send {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error>;

    fn validate(&self) -> Result<(), crate::Error> {
        Ok(())
    }

    fn required_permission(&self) -> Option<Permissions> {
        None
    }
}

#[allow(dead_code)]
pub enum Effect {
    Global(ServerMessage),
    Others(Snowflake, ServerMessage),
    Direct(Snowflake, ServerMessage),
}

pub enum CommandResult {
    Effects(Vec<Effect>),
    Silent,
}

impl From<ServerMessage> for CommandResult {
    fn from(msg: ServerMessage) -> Self {
        CommandResult::Effects(vec![Effect::Global(msg)])
    }
}
