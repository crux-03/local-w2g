use thiserror::Error;

use crate::{Snowflake, services::permissions::Permissions};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid User")]
    InvalidUser,
    #[error("Missing one or more permissions: {0}")]
    NoPermission(Permissions),
    #[error("Message not found")]
    MessageNotFound,
    #[error("Message is not a widget")]
    NotAWidget,
    #[error("Cannot update an already completed widget")]
    WidgetAlreadyDone,
    #[error("An internal error occured: {0}")]
    Internal(String),
    #[error("Command validation failed ({0}): {1}")]
    CommandValidation(String, String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Video does not exist: {0}")]
    InvalidVideo(Snowflake),
    
}
