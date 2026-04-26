use anyhow::Ok;

use crate::{
    commands::{
        Command,
        download::{DownloadDoneCommand, DownloadProgressCommand},
        messages::{MessageHistoryCommand, SendMessageCommand},
        misc::PingCommand,
        playback::{
            PausePlaybackCommand, PlayCommand, ResumePlaybackCommand, SeekCommand,
            SelectVideoCommand,
        },
        playlist::{
            RequestPlaylistCommand, SetEntryAudioTrackCommand, SetEntryDisplayNameCommand,
            SetEntrySubtitleTrackCommand, SwapEntriesCommand,
        },
        resync::{InitiateResyncCommand, SendResyncReportCommand},
        state::{
            AssertPendingCommand, AssertReadyBulkCommand, AssertReadyCommand,
            ConfirmReadyForPlayCommand, HeartbeatCommand,
        },
        user::{EditPermissionCommand, IdentifySelfCommand, ListUsersCommand},
    },
    websocket::ClientMessage,
};

pub fn parse_client_message(msg: &str) -> anyhow::Result<Box<dyn Command>> {
    let parsed: ClientMessage = serde_json::from_str(msg)?;

    match parsed {
        ClientMessage::Ping => Ok(Box::new(PingCommand)),
        ClientMessage::RequestIdentity => Ok(Box::new(IdentifySelfCommand)),
        ClientMessage::RequestUsers => Ok(Box::new(ListUsersCommand)),
        ClientMessage::EditUserPermissions {
            target_user,
            permission,
            granted,
        } => Ok(Box::new(EditPermissionCommand {
            target_user,
            permission,
            granted,
        })),
        ClientMessage::SendMessage { content } => Ok(Box::new(SendMessageCommand { content })),
        ClientMessage::RequestMessageHistory => Ok(Box::new(MessageHistoryCommand)),
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
        ClientMessage::AssertReadyBulk { on_device } => {
            Ok(Box::new(AssertReadyBulkCommand { on_device }))
        }
        ClientMessage::AssertPending { video_id } => {
            Ok(Box::new(AssertPendingCommand { video_id }))
        }
        ClientMessage::Heartbeat => Ok(Box::new(HeartbeatCommand)),
        ClientMessage::ConfirmReadyForPlay { request_id } => {
            Ok(Box::new(ConfirmReadyForPlayCommand { request_id }))
        }
        ClientMessage::Play => Ok(Box::new(PlayCommand)),
        ClientMessage::RequestPause => Ok(Box::new(PausePlaybackCommand)),
        ClientMessage::RequestResume => Ok(Box::new(ResumePlaybackCommand)),
        ClientMessage::RequestSeek { timestamp } => Ok(Box::new(SeekCommand { timestamp })),
        ClientMessage::RequestPlaylist => Ok(Box::new(RequestPlaylistCommand)),
        ClientMessage::SelectVideo { video_id } => Ok(Box::new(SelectVideoCommand { video_id })),
        ClientMessage::SwapEntries { first, second } => {
            Ok(Box::new(SwapEntriesCommand { first, second }))
        }
        ClientMessage::SetAudioTrack {
            video_id,
            audio_track,
        } => Ok(Box::new(SetEntryAudioTrackCommand {
            video_id,
            audio_track,
        })),
        ClientMessage::SetDisplayName {
            video_id,
            display_name,
        } => Ok(Box::new(SetEntryDisplayNameCommand {
            video_id,
            display_name,
        })),
        ClientMessage::SetSubtitleTrack {
            video_id,
            subtitle_track,
        } => Ok(Box::new(SetEntrySubtitleTrackCommand {
            video_id,
            subtitle_track,
        })),
    }
}
