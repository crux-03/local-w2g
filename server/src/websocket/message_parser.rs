use anyhow::Ok;

use crate::{
    commands::{
        Command,
        download::{DownloadDoneCommand, DownloadProgressCommand},
        messages::SendMessageCommand,
        playback::{PlayCommand, SelectVideoCommand},
        resync::{InitiateResyncCommand, SendResyncReportCommand},
        state::{AssertReadyCommand, ConfirmReadyForPlayCommand, HeartbeatCommand},
    },
    websocket::ClientMessage,
};

pub fn parse_client_message(msg: &str) -> anyhow::Result<Box<dyn Command>> {
    let parsed: ClientMessage = serde_json::from_str(msg)?;

    match parsed {
        ClientMessage::SendMessage { content } => Ok(Box::new(SendMessageCommand { content })),
        ClientMessage::StartResync => Ok(Box::new(InitiateResyncCommand)),
        ClientMessage::SendResyncReport {
            state_id,
            timestamp,
        } => Ok(Box::new(SendResyncReportCommand {
            state_id,
            timestamp,
        })),
        ClientMessage::DownloadProgress {
            widget_id,
            bytes_done,
        } => Ok(Box::new(DownloadProgressCommand {
            widget_id,
            bytes_done,
        })),
        ClientMessage::DownloadDone { widget_id } => {
            Ok(Box::new(DownloadDoneCommand { widget_id }))
        }
        ClientMessage::AssertReady {
            video_id,
            on_device,
        } => Ok(Box::new(AssertReadyCommand {
            video_id,
            on_device,
        })),
        ClientMessage::Heartbeat => Ok(Box::new(HeartbeatCommand)),
        ClientMessage::ConfirmReadyForPlay { request_id } => {
            Ok(Box::new(ConfirmReadyForPlayCommand { request_id }))
        }
        ClientMessage::Play => Ok(Box::new(PlayCommand)),
        ClientMessage::SelectVideo { video_id } => Ok(Box::new(SelectVideoCommand { video_id })),
    }
}
