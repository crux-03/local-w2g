use bitflags::bitflags;

mod service;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use service::PermissionService;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: i64 {
        const MANAGE_PLAYBACK = 1 << 0;
        const SEND_MESSAGE = 1 << 1;
        const MANAGE_USERS = 1 << 2;
        const SEND_STATE = 1 << 3;
        const MANAGE_MEDIA = 1 << 4;
    }
}

impl Permissions {
    pub fn admin() -> Permissions {
        Permissions::all()
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions::SEND_MESSAGE | Permissions::SEND_STATE
    }
}

impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = i64::deserialize(deserializer)?;
        Ok(Permissions::from_bits_truncate(bits))
    }
}

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.iter_names().map(|(name, _)| name))
            .finish()
    }
}
