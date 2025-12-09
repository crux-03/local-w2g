use std::sync::Arc;

use axum::extract::ws::Message;
use uuid::Uuid;
use dto::*;

use crate::{dto, state::AppState};

async fn handle_pause(user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Check permission
    if !state.has_permission(&user_id, "pause").await {
        let error = ServerMessage::Error { 
            message: "You don't have permission to pause".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    let response = ServerMessage::Pause;
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast(&user_id, Message::Text(bytes)).await;
    
    // Log activity
    let clients = state.clients.read().await;
    let username = clients.get(&user_id).and_then(|c| c.username.clone());
    drop(clients);
    state.log_activity(user_id, username, "paused playback".to_string()).await;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_play(user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Check permission
    if !state.has_permission(&user_id, "pause").await {
        let error = ServerMessage::Error { 
            message: "You don't have permission to play".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    let response = ServerMessage::Play;
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast(&user_id, Message::Text(bytes)).await;
    
    // Log activity
    let clients = state.clients.read().await;
    let username = clients.get(&user_id).and_then(|c| c.username.clone());
    drop(clients);
    state.log_activity(user_id, username, "resumed playback".to_string()).await;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_seek(position: f64, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Check permission
    if !state.has_permission(&user_id, "seek").await {
        let error = ServerMessage::Error { 
            message: "You don't have permission to seek".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    let response = ServerMessage::Seek { position };
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast(&user_id, Message::Text(bytes)).await;
    
    // Log activity
    let clients = state.clients.read().await;
    let username = clients.get(&user_id).and_then(|c| c.username.clone());
    drop(clients);
    let time = format_time(position);
    state.log_activity(user_id, username, format!("seeked to {}", time)).await;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_subtitle_track(index: u32, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Check permission
    if !state.has_permission(&user_id, "subtitle").await {
        let error = ServerMessage::Error { 
            message: "You don't have permission to change subtitles".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    let response = ServerMessage::SubtitleTrack { index };
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast(&user_id, Message::Text(bytes)).await;
    
    // Log activity
    let clients = state.clients.read().await;
    let username = clients.get(&user_id).and_then(|c| c.username.clone());
    drop(clients);
    state.log_activity(user_id, username, format!("changed subtitle track to {}", index)).await;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_audio_track(index: u32, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Check permission
    if !state.has_permission(&user_id, "audio").await {
        let error = ServerMessage::Error { 
            message: "You don't have permission to change audio".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    let response = ServerMessage::AudioTrack { index };
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast(&user_id, Message::Text(bytes)).await;
    
    // Log activity
    let clients = state.clients.read().await;
    let username = clients.get(&user_id).and_then(|c| c.username.clone());
    drop(clients);
    state.log_activity(user_id, username, format!("changed audio track to {}", index)).await;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_ready(ready: bool, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    state.set_ready(user_id, ready).await;
    
    let response = ServerMessage::Ready { client_id: user_id, value: ready };
    let bytes = serde_json::to_string(&response)?.into();

    state.broadcast_all(Message::Text(bytes)).await;
    
    // Check if all clients are ready
    if state.all_ready().await {
        let all_ready_msg = ServerMessage::AllReady;
        let bytes = serde_json::to_string(&all_ready_msg)?.into();
        state.broadcast_all(Message::Text(bytes)).await;
    }
    
    // Send updated user list
    send_user_update(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_set_username(username: String, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    state.set_username(user_id, username).await;
    
    // Send updated user list to all clients
    send_user_update(Arc::clone(&state)).await?;
    
    // Send updated logs to all clients
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_set_permission(
    target_id: String, 
    permission: String, 
    value: bool,
    user_id: Uuid, 
    state: Arc<AppState>
) -> anyhow::Result<()> {
    // Only owner can set permissions
    if !state.is_owner(&user_id).await {
        let error = ServerMessage::Error { 
            message: "Only the owner can change permissions".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    // Parse target UUID
    let target_uuid = Uuid::parse_str(&target_id)?;
    
    // Can't change owner's permissions
    if state.is_owner(&target_uuid).await {
        let error = ServerMessage::Error { 
            message: "Cannot change owner's permissions".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    state.set_permission(target_uuid, &permission, value).await;
    
    // Send updated permissions to all clients
    send_permissions_update(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_transfer_ownership(
    target_id: String,
    user_id: Uuid,
    state: Arc<AppState>
) -> anyhow::Result<()> {
    // Only owner can transfer ownership
    if !state.is_owner(&user_id).await {
        let error = ServerMessage::Error { 
            message: "Only the owner can transfer ownership".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    // Parse target UUID
    let target_uuid = Uuid::parse_str(&target_id)?;
    
    if state.transfer_ownership(target_uuid).await {
        let response = ServerMessage::OwnershipTransferred { new_owner_id: target_uuid };
        let bytes = serde_json::to_string(&response)?.into();
        state.broadcast_all(Message::Text(bytes)).await;
        
        // Send updated user list and permissions
        send_user_update(Arc::clone(&state)).await?;
        send_permissions_update(Arc::clone(&state)).await?;
        
        // Log activity
        let clients = state.clients.read().await;
        let old_owner_name = clients.get(&user_id).and_then(|c| c.username.clone());
        let new_owner_name = clients.get(&target_uuid).and_then(|c| c.username.clone());
        drop(clients);
        
        state.log_activity(
            user_id, 
            old_owner_name.clone(), 
            format!("transferred ownership to {}", new_owner_name.unwrap_or_else(|| target_uuid.to_string()))
        ).await;
        
        send_activity_logs(Arc::clone(&state)).await?;
    } else {
        let error = ServerMessage::Error { 
            message: "Failed to transfer ownership".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
    }
    
    Ok(())
}

async fn handle_select_video(
    index: usize,
    user_id: Uuid,
    state: Arc<AppState>
) -> anyhow::Result<()> {
    // Only owner can select video
    if !state.is_owner(&user_id).await {
        let error = ServerMessage::Error { 
            message: "Only the owner can select videos".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
        return Ok(());
    }

    if state.set_current_video(index).await {
        // Send updated playlist to all clients
        send_playlist_update(Arc::clone(&state)).await?;
        
        // Log activity
        let playlist = state.get_playlist().await;
        if let Some(video) = playlist.get(index) {
            let clients = state.clients.read().await;
            let username = clients.get(&user_id).and_then(|c| c.username.clone());
            drop(clients);
            state.log_activity(user_id, username, format!("selected video: {}", video.filename)).await;
            send_activity_logs(Arc::clone(&state)).await?;
        }
    } else {
        let error = ServerMessage::Error { 
            message: "Invalid video index".to_string() 
        };
        let bytes = serde_json::to_string(&error)?.into();
        state.send_to_client(&user_id, Message::Text(bytes)).await;
    }
    
    Ok(())
}

/// Send user list update to all clients
pub async fn send_user_update(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut users: Vec<dto::User> = Vec::new();
    
    let clients = state.clients.read().await;
    
    for (client_id, client) in clients.iter() {
        let user = dto::User {
            id: client_id.to_string(),
            username: client.username.clone(),
            is_owner: state.is_owner(client_id).await,
            is_ready: client.is_ready,
            status: if client.is_ready { UserStatus::Ready } else { UserStatus::Waiting },
        };
        users.push(user);
    }
    
    let response = ServerMessage::UserUpdate { users };
    let bytes = serde_json::to_string(&response)?.into();
    
    state.broadcast_all(Message::Text(bytes)).await;
    Ok(())
}

/// Send permissions update to all clients
pub async fn send_permissions_update(state: Arc<AppState>) -> anyhow::Result<()> {
    let all_permissions = state.get_all_permissions().await;
    
    let permissions: Vec<UserPermission> = all_permissions
        .iter()
        .map(|(id, perms)| UserPermission {
            user_id: id.to_string(),
            allow_pause: perms.allow_pause,
            allow_seek: perms.allow_seek,
            allow_subtitle: perms.allow_subtitle,
            allow_audio: perms.allow_audio,
        })
        .collect();
    
    let response = ServerMessage::PermissionsUpdate { permissions };
    let bytes = serde_json::to_string(&response)?.into();
    
    state.broadcast_all(Message::Text(bytes)).await;
    Ok(())
}

/// Send playlist update to all clients
pub async fn send_playlist_update(state: Arc<AppState>) -> anyhow::Result<()> {
    let playlist = state.get_playlist().await;
    let current_index = *state.current_video_index.read().await;
    
    let videos: Vec<Video> = playlist
        .iter()
        .map(|v| Video {
            id: v.id.to_string(),
            filename: v.filename.clone(),
            size_bytes: v.size_bytes,
            size_display: format_size(v.size_bytes),
            uploaded_at: v.uploaded_at,
            uploader_id: v.uploader_id.to_string(),
        })
        .collect();
    
    let response = ServerMessage::PlaylistUpdate {
        videos,
        current_index,
    };
    let bytes = serde_json::to_string(&response)?.into();
    
    state.broadcast_all(Message::Text(bytes)).await;
    Ok(())
}

/// Send activity logs to all clients
pub async fn send_activity_logs(state: Arc<AppState>) -> anyhow::Result<()> {
    let logs = state.get_recent_logs(20).await;
    
    let log_entries: Vec<LogEntry> = logs
        .iter()
        .map(|log| LogEntry {
            timestamp: log.timestamp,
            user_id: log.user_id.to_string(),
            username: log.username.clone(),
            action: log.action.clone(),
            source: log.source.clone(),
        })
        .collect();
    
    let response = ServerMessage::ActivityLog { logs: log_entries };
    let bytes = serde_json::to_string(&response)?.into();
    
    state.broadcast_all(Message::Text(bytes)).await;
    Ok(())
}

pub async fn send_initial_message(user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    let is_owner = state.is_owner(&user_id).await;
    
    // Send connected message
    let response = ServerMessage::Connected { 
        client_id: user_id, 
        is_owner 
    };
    let bytes = serde_json::to_string(&response)?.into();
    state.send_to_client(&user_id, Message::Text(bytes)).await;
    
    // Send current state to new client
    send_user_update(Arc::clone(&state)).await?;
    send_permissions_update(Arc::clone(&state)).await?;
    send_playlist_update(Arc::clone(&state)).await?;
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

async fn handle_request_state(user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    // Re-send all current state to the requesting client
    // This is used when a client's event listeners are set up after initial connection
    send_user_update(Arc::clone(&state)).await?;
    send_permissions_update(Arc::clone(&state)).await?;
    send_playlist_update(Arc::clone(&state)).await?;
    send_activity_logs(Arc::clone(&state)).await?;
    
    Ok(())
}

pub async fn handle_command(message: &str, user_id: Uuid, state: Arc<AppState>) -> anyhow::Result<()> {
    let message: ClientMessage = serde_json::from_str(message)?;

    match message {
        ClientMessage::Pause => handle_pause(user_id, state).await,
        ClientMessage::Play => handle_play(user_id, state).await,
        ClientMessage::Seek { position } => handle_seek(position, user_id, state).await,
        ClientMessage::SubtitleTrack { index } => handle_subtitle_track(index, user_id, state).await,
        ClientMessage::AudioTrack { index } => handle_audio_track(index, user_id, state).await,
        ClientMessage::Ready { value } => handle_ready(value, user_id, state).await,
        ClientMessage::SetUsername { username } => handle_set_username(username, user_id, state).await,
        ClientMessage::SetPermission { client_id, permission, value } => {
            handle_set_permission(client_id, permission, value, user_id, state).await
        },
        ClientMessage::TransferOwnership { client_id } => {
            handle_transfer_ownership(client_id, user_id, state).await
        },
        ClientMessage::SelectVideo { index } => {
            handle_select_video(index, user_id, state).await
        },
        ClientMessage::RequestState => {
            handle_request_state(user_id, state).await
        },
    }
}

/// Helper function to format seconds into MM:SS or HH:MM:SS
fn format_time(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}