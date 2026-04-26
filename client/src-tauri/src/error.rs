use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ws channel closed")]
    ChannelClosed,
}

pub type Result<T> = std::result::Result<T, Error>;
