use bitflags::bitflags;

mod service;

use serde::Serialize;
pub use service::PermissionService;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct Permissions: i64 {
        const MANAGE_PLAYBACK = 1 << 0;
        const SEND_MESSAGE = 1 << 1;
        const MANAGE_USERS = 1 << 2;
        const SEND_STATE = 1 << 3;
        const MANAGE_PLAYLIST = 1 << 4;
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

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.iter_names().map(|(name, _)| name))
            .finish()
    }
}
