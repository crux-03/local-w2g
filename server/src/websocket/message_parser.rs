use crate::{
    commands::{
        Command,
        messages::SendMessageCommand,
        resync::{InitiateResyncCommand, SendResyncReportCommand},
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
    }
}
