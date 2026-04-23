use std::sync::Arc;

use crate::Snowflake;

use crate::commands::Effect;
use crate::{
    commands::{Command, CommandResult},
    core::AppState
};

pub async fn execute_command(
    cmd: Box<dyn Command>,
    user_id: Snowflake,
    state: Arc<AppState>,
) -> Result<(), crate::Error> {
    cmd.validate()?;

    if let Some(required) = cmd.required_permission() {
        state
            .services()
            .permission()
            .require(&user_id, required)
            .await?;
    }

    let result = cmd.execute(state.clone(), user_id).await?;

    if let CommandResult::Effects(effects) = result {
        for effect in effects {
            apply_effect(&state, effect).await?;
        }
    }

    Ok(())
}

pub async fn apply_effect(state: &AppState, effect: Effect) -> Result<(), crate::Error> {
    match effect {
        Effect::Global(msg) => {
            state.broadcast(serde_json::to_string(&msg)?.into()).await;
        }
        Effect::Others(except_id, msg) => {
            state
                .broadcast_except(except_id, serde_json::to_string(&msg)?.into())
                .await;
        }
        Effect::Direct(id, msg) => {
            state
                .send_to_client(&id, serde_json::to_string(&msg)?.into())
                .await;
        }
    }
    Ok(())
}
