use crate::{
    commands::{Command, messages::SendMessageCommand},
    websocket::ClientMessage,
};

pub fn parse_client_message(msg: &str) -> anyhow::Result<Box<dyn Command>> {
    let parsed: ClientMessage = serde_json::from_str(msg)?;

    match parsed {
        ClientMessage::SendMessage { content } => Ok(Box::new(SendMessageCommand { content })),
    }
}
