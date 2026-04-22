use serde::Serialize;

use crate::{Snowflake, services::permissions::Permissions};

#[derive(Clone, Serialize)]
pub struct User {
    pub id: Snowflake,
    pub display_name: Option<String>,
    pub permissions: Permissions
}