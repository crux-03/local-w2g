use axum::extract::ws::Message;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};

use crate::config::Config;

pub type ClientId = Uuid;
pub type ClientSender = mpsc::UnboundedSender<Message>;

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub _id: ClientId,
    pub username: Option<String>,
    pub sender: ClientSender,
    pub is_ready: bool,
    pub _connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClientPermissions {
    pub allow_pause: bool,
    pub allow_seek: bool,
    pub allow_subtitle: bool,
    pub allow_audio: bool,
}

impl Default for ClientPermissions {
    fn default() -> Self {
        Self {
            allow_pause: false,
            allow_seek: false,
            allow_subtitle: false,
            allow_audio: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub id: Uuid,
    pub filename: String,
    pub size_bytes: u64,
    pub _path: String,
    pub uploaded_at: DateTime<Utc>,
    pub uploader_id: ClientId,
}

#[derive(Debug, Clone)]
pub struct ActivityLog {
    pub timestamp: DateTime<Utc>,
    pub user_id: ClientId,
    pub username: Option<String>,
    pub action: String,
    pub source: crate::dto::LogSource,
}

pub struct AppState {
    /// Configuration
    pub config: Config,

    /// Shared access password
    pub access_password: String,

    /// Connected clients with their WebSocket senders
    pub clients: RwLock<HashMap<ClientId, ClientInfo>>,

    /// The owner (first connected client)
    pub owner_id: RwLock<Option<ClientId>>,

    /// Permissions for each client (only owner can modify)
    pub permissions: RwLock<HashMap<ClientId, ClientPermissions>>,

    /// Playlist of videos
    pub playlist: RwLock<Vec<VideoInfo>>,

    /// Current video being played
    pub current_video_index: RwLock<Option<usize>>,

    /// Activity log
    pub activity_log: RwLock<Vec<ActivityLog>>,
}


impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            access_password: generate_password(),
            clients: RwLock::new(HashMap::new()),
            owner_id: RwLock::new(None),
            permissions: RwLock::new(HashMap::new()),
            playlist: RwLock::new(Vec::new()),
            current_video_index: RwLock::new(None),
            activity_log: RwLock::new(Vec::new()),
        }
    }

    /// Add a new client connection
    pub async fn add_client(&self, id: ClientId, sender: ClientSender) {
        let mut clients = self.clients.write().await;

        // Set as owner if first client
        let mut owner = self.owner_id.write().await;
        let is_owner = if owner.is_none() {
            *owner = Some(id);
            tracing::info!("Client {} is now the owner", id);
            true
        } else {
            false
        };

        clients.insert(
            id,
            ClientInfo {
                _id: id,
                username: None,
                sender,
                is_ready: false,
                _connected_at: Utc::now(),
            },
        );

        // Set default permissions (owner gets full permissions)
        let mut permissions = self.permissions.write().await;
        if is_owner {
            permissions.insert(id, ClientPermissions {
                allow_pause: true,
                allow_seek: true,
                allow_subtitle: true,
                allow_audio: true,
            });
        } else {
            permissions.insert(id, ClientPermissions::default());
        }

        tracing::info!("Client {} connected. Total clients: {}", id, clients.len());

        // Log activity
        self.log_activity(id, None, "connected".to_string()).await;
    }

    /// Remove a client connection
    pub async fn remove_client(&self, id: &ClientId) {
        let mut clients = self.clients.write().await;
        let username = clients.get(id).and_then(|c| c.username.clone());
        clients.remove(id);

        let mut permissions = self.permissions.write().await;
        permissions.remove(id);

        // If owner disconnects, transfer to next client
        let mut owner = self.owner_id.write().await;
        if owner.as_ref() == Some(id) {
            *owner = clients.keys().next().cloned();
            if let Some(new_owner) = &*owner {
                tracing::info!("New owner: {}", new_owner);
                // Grant full permissions to new owner
                if let Some(perms) = permissions.get_mut(new_owner) {
                    *perms = ClientPermissions {
                        allow_pause: true,
                        allow_seek: true,
                        allow_subtitle: true,
                        allow_audio: true,
                    };
                }
            } else {
                tracing::info!("No owner (all clients disconnected)");
            }
        }

        tracing::info!("Client {} disconnected. Total clients: {}", id, clients.len());

        // Log activity
        self.log_activity(*id, username, "disconnected".to_string()).await;
    }

    /// Broadcast a message to all connected clients except sender
    pub async fn broadcast(&self, sender_id: &ClientId, message: Message) {
        let clients = self.clients.read().await;

        for (id, client) in clients.iter() {
            if id != sender_id {
                let _ = client.sender.send(message.clone());
            }
        }
    }

    /// Broadcast a message to all connected clients including sender
    pub async fn broadcast_all(&self, message: Message) {
        let clients = self.clients.read().await;

        for client in clients.values() {
            let _ = client.sender.send(message.clone());
        }
    }

    /// Send a message to a specific client
    pub async fn send_to_client(&self, client_id: &ClientId, message: Message) {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            let _ = client.sender.send(message);
        }
    }

    /// Check if a client is the owner
    pub async fn is_owner(&self, id: &ClientId) -> bool {
        let owner = self.owner_id.read().await;
        owner.as_ref() == Some(id)
    }

    /// Check if all clients are ready
    pub async fn all_ready(&self) -> bool {
        let clients = self.clients.read().await;

        if clients.is_empty() {
            return false;
        }

        clients.values().all(|client| client.is_ready)
    }

    /// Set username for a client
    pub async fn set_username(&self, client_id: ClientId, username: String) {
        let mut clients = self.clients.write().await;

        // Get old username before changing
        let old_username = clients.get(&client_id).and_then(|c| c.username.clone());

        if let Some(client) = clients.get_mut(&client_id) {
            client.username = Some(username.clone());
        }

        // Log activity with old username
        drop(clients);
        let log_message = if let Some(old_name) = old_username {
            format!("changed username from {} to {}", old_name, username)
        } else {
            format!("set username to {}", username)
        };
        self.log_activity(client_id, Some(username), log_message).await;
    }

    /// Set ready state for a client
    pub async fn set_ready(&self, client_id: ClientId, ready: bool) {
        let mut clients = self.clients.write().await;

        if let Some(client) = clients.get_mut(&client_id) {
            client.is_ready = ready;
        }
    }

    /// Check if client has a specific permission
    pub async fn has_permission(&self, client_id: &ClientId, permission: &str) -> bool {
        // Owner always has all permissions
        if self.is_owner(client_id).await {
            return true;
        }

        let permissions = self.permissions.read().await;
        if let Some(perms) = permissions.get(client_id) {
            match permission {
                "pause" => perms.allow_pause,
                "seek" => perms.allow_seek,
                "subtitle" => perms.allow_subtitle,
                "audio" => perms.allow_audio,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Set permission for a client (only owner can do this)
    pub async fn set_permission(&self, client_id: ClientId, permission: &str, value: bool) {
        let mut permissions = self.permissions.write().await;

        if let Some(perms) = permissions.get_mut(&client_id) {
            match permission {
                "pause" => perms.allow_pause = value,
                "seek" => perms.allow_seek = value,
                "subtitle" => perms.allow_subtitle = value,
                "audio" => perms.allow_audio = value,
                _ => {},
            }
        }
    }

    /// Add video to playlist
    pub async fn add_video(&self, video: VideoInfo) {
        let mut playlist = self.playlist.write().await;
        playlist.push(video.clone());

        // If this is the first video, set it as current
        if playlist.len() == 1 {
            let mut current = self.current_video_index.write().await;
            *current = Some(0);
        }

        // Log activity
        let clients = self.clients.read().await;
        let username = clients.get(&video.uploader_id).and_then(|c| c.username.clone());
        drop(clients);
        drop(playlist);
        self.log_activity(video.uploader_id, username, format!("uploaded {}", video.filename)).await;
    }

    /// Get current video
    pub async fn _get_current_video(&self) -> Option<VideoInfo> {
        let playlist = self.playlist.read().await;
        let current_idx = self.current_video_index.read().await;

        if let Some(idx) = *current_idx {
            playlist.get(idx).cloned()
        } else {
            None
        }
    }

    /// Get all videos in playlist
    pub async fn get_playlist(&self) -> Vec<VideoInfo> {
        let playlist = self.playlist.read().await;
        playlist.clone()
    }

    /// Set current video by index
    pub async fn set_current_video(&self, index: usize) -> bool {
        let playlist = self.playlist.read().await;
        if index < playlist.len() {
            let mut current = self.current_video_index.write().await;
            *current = Some(index);
            true
        } else {
            false
        }
    }

    /// Log an activity
    pub async fn log_activity(&self, user_id: ClientId, username: Option<String>, action: String) {
        let mut log = self.activity_log.write().await;
        log.push(ActivityLog {
            timestamp: Utc::now(),
            user_id,
            username,
            action,
            source: crate::dto::LogSource::Server,
        });

        // Keep only last 100 entries
        if log.len() > 100 {
            log.remove(0);
        }
    }

    /// Get recent activity logs
    pub async fn get_recent_logs(&self, count: usize) -> Vec<ActivityLog> {
        let log = self.activity_log.read().await;
        let start = if log.len() > count { log.len() - count } else { 0 };
        log[start..].to_vec()
    }

    /// Get all client permissions
    pub async fn get_all_permissions(&self) -> HashMap<ClientId, ClientPermissions> {
        let permissions = self.permissions.read().await;
        permissions.clone()
    }

    /// Transfer ownership to another client
    pub async fn transfer_ownership(&self, new_owner_id: ClientId) -> bool {
        let clients = self.clients.read().await;

        if clients.contains_key(&new_owner_id) {
            let mut owner = self.owner_id.write().await;
            let old_owner = *owner;
            *owner = Some(new_owner_id);

            // Update permissions
            let mut permissions = self.permissions.write().await;

            // Remove old owner's full permissions
            if let Some(old_id) = old_owner {
                if let Some(perms) = permissions.get_mut(&old_id) {
                    *perms = ClientPermissions::default();
                }
            }

            // Grant new owner full permissions
            if let Some(perms) = permissions.get_mut(&new_owner_id) {
                *perms = ClientPermissions {
                    allow_pause: true,
                    allow_seek: true,
                    allow_subtitle: true,
                    allow_audio: true,
                };
            }

            tracing::info!("Ownership transferred to {}", new_owner_id);
            true
        } else {
            false
        }
    }
}

use rand::{distr::Alphanumeric, Rng};

fn generate_password() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
