use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    services::{message::WidgetState, permissions::Permissions},
    websocket::ServerMessage,
};

pub struct UpdateWidgetCommand {
    pub msg_id: Snowflake,
    pub state: WidgetState,
    pub finished: bool,
}

#[async_trait]
impl Command for UpdateWidgetCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        _user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let message_service = state.services().message();
        match self.finished {
            true => {
                let entry = message_service
                    .finish_widget(self.msg_id, self.state.clone())
                    .await?;

                Ok(ServerMessage::WidgetDone {
                    entry,
                }.into())
            }
            false => {
                let entry = message_service
                    .update_widget(self.msg_id, self.state.clone())
                    .await?;

                Ok(ServerMessage::WidgetUpdated {
                    entry,
                }.into())
            }
        }
    }

    fn validate(&self) -> Result<(), crate::Error> {
        Ok(())
    }

    fn required_permission(&self) -> Option<Permissions> {
        None
    }
}
