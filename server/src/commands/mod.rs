pub mod handler;
pub mod messages;
pub mod resync;

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

    fn broadcast_scope(&self) -> BroadcastScope;
}

pub enum CommandResult {
    Broadcast(ServerMessage),
    SendToClient(ServerMessage),
    Silent,
}

pub enum BroadcastScope {
    Global,
    Others(Snowflake),
    Direct(Snowflake),
    Silent,
}
