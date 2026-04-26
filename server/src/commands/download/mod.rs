use std::sync::Arc;

use async_trait::async_trait;

use crate::{Snowflake, commands::{Command, CommandResult}, core::AppState, services::message::{EntryKind, WidgetState}, websocket::ServerMessage};

async fn load_download_widget(
    state: &AppState,
    widget_id: Snowflake,
    user_id: Snowflake,
) -> Result<(Snowflake, String, u64), crate::Error> {
    let entry = state
        .services()
        .message()
        .get(widget_id)
        .await
        .ok_or(crate::Error::MessageNotFound)?;

    let (reporter, filename, bytes_total) = match entry.kind {
        EntryKind::Widget {
            state:
                WidgetState::Download {
                    reporter,
                    filename,
                    bytes_total,
                    ..
                },
            ..
        } => (reporter, filename, bytes_total),
        _ => return Err(crate::Error::NotAWidget),
    };

    if reporter != user_id {
        return Err(crate::Error::Forbidden("not your widget".into()));
    }
    Ok((reporter, filename, bytes_total))
}

pub struct DownloadProgressCommand {
    pub widget_id: Snowflake,
    pub bytes_done: u64,
}

#[async_trait]
impl Command for DownloadProgressCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let (reporter, filename, bytes_total) =
            load_download_widget(&state, self.widget_id, user_id).await?;

        let new_state = WidgetState::Download {
            reporter,
            filename,
            bytes_done: self.bytes_done.min(bytes_total),
            bytes_total,
        };

        let entry = state
            .services()
            .message()
            .update_widget(self.widget_id, new_state)
            .await?;

        Ok(ServerMessage::WidgetUpdated { entry }.into())
    }
}

pub struct DownloadDoneCommand {
    pub widget_id: Snowflake,
}

#[async_trait]
impl Command for DownloadDoneCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let (reporter, filename, bytes_total) =
            load_download_widget(&state, self.widget_id, user_id).await?;

        let final_state = WidgetState::Download {
            reporter,
            filename,
            bytes_done: bytes_total,
            bytes_total,
        };

        let entry = state
            .services()
            .message()
            .finish_widget(self.widget_id, final_state)
            .await?;

        Ok(ServerMessage::WidgetDone { entry }.into())
    }
}
