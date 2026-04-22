use std::sync::Arc;

use crate::Snowflake;

use crate::commands::BroadcastScope;
use crate::{
    commands::{Command, CommandResult},
    core::AppState,
    websocket::ServerMessage,
};

pub async fn execute_command(
    cmd: Box<dyn Command>,
    user_id: Snowflake,
    state: Arc<AppState>,
) -> Result<(), crate::Error> {
    // Validate input
    cmd.validate()?;

    if let Some(required) = cmd.required_permission() {
        state
            .services()
            .permission()
            .require(&user_id, required)
            .await?;
    }

    // Execute
    let result = cmd.execute(state.clone(), user_id).await?;

    // Broadcast result
    match result {
        CommandResult::Broadcast(msg) => {
            broadcast_to_scope(Arc::clone(&state), cmd.broadcast_scope(), msg).await?;
        }
        CommandResult::SendToClient(msg) => {
            state
                .send_to_client(&user_id, serde_json::to_string(&msg)?.into())
                .await;
        }
        CommandResult::Silent => {}
    }

    Ok(())
}

async fn broadcast_to_scope(
    state: Arc<AppState>,
    scope: BroadcastScope,
    msg: ServerMessage,
) -> Result<(), crate::Error> {
    let json = serde_json::to_string(&msg)?.into();

    match scope {
        BroadcastScope::Global => {
            state.broadcast(json).await;
        }
        BroadcastScope::Others(except_id) => {
            state.broadcast_except(except_id, json).await;
        }
        BroadcastScope::Direct(id) => state.send_to_client(&id, json).await,
        BroadcastScope::Silent => {
            tracing::warn!("Tried broadcasting to unimplemented BroadcastScope")
        }
    }

    Ok(())
}
