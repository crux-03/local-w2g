use crate::types::{ClientMessage, CommandResult, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use reqwest_websocket::{Message as WsMessage, RequestBuilderExt};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, RwLock};

pub struct WebSocketClient {
    sender: Arc<RwLock<Option<mpsc::UnboundedSender<ClientMessage>>>>,
    is_connected: Arc<RwLock<bool>>,
}

impl WebSocketClient {
    pub fn new() -> Self {
        Self {
            sender: Arc::new(RwLock::new(None)),
            is_connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(
        &mut self,
        url: &str,
        app_handle: AppHandle,
        client_id_ref: Arc<RwLock<Option<String>>>,
        is_owner_ref: Arc<RwLock<bool>>,
        password: String
    ) -> CommandResult<()> {
        // Check if already connected
        if *self.is_connected.read().await {
            return Err("Already connected".to_string());
        }

        // Build WebSocket URL
        let ws_url = if url.starts_with("ws://") || url.starts_with("wss://") {
            url.to_string()
        } else {
            let base = url.trim_end_matches('/');
            if url.starts_with("https://") {
                format!("{}/ws", base.replace("https://", "wss://"))
            } else if url.starts_with("http://") {
                format!("{}/ws", base.replace("http://", "ws://"))
            } else {
                format!("ws://{}/ws", base)
            }
        };

        // Connect to WebSocket
        let response = reqwest::Client::default()
            .get(&ws_url)
            .header("X-Access-Key", password)
            .upgrade()
            .send()
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        let websocket = response
            .into_websocket()
            .await
            .map_err(|e| format!("Failed to upgrade connection: {}", e))?;

        // Create channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<ClientMessage>();
        *self.sender.write().await = Some(tx);

        let (mut ws_tx, mut ws_rx) = websocket.split();

        // Update state
        *self.is_connected.write().await = true;

        // Spawn task to send messages from channel to WebSocket
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if ws_tx.send(WsMessage::Text(json)).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to receive messages from WebSocket
        let app_handle_clone = app_handle.clone();
        let is_connected = self.is_connected.clone();
        tokio::spawn(async move {
            loop {
                match ws_rx.next().await {
                    Some(Ok(msg)) => {
                        match msg {
                    WsMessage::Text(text) => {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                            // Handle Connected message specially to store client_id and is_owner
                            if let ServerMessage::Connected { ref client_id, is_owner } = server_msg {
                                *client_id_ref.write().await = Some(client_id.clone());
                                *is_owner_ref.write().await = is_owner;
                            }
                            
                            // Handle OwnershipTransferred message
                            if let ServerMessage::OwnershipTransferred { ref new_owner_id } = server_msg {
                                let current_id = client_id_ref.read().await;
                                if let Some(id) = current_id.as_ref() {
                                    *is_owner_ref.write().await = id == new_owner_id;
                                }
                            }
                            
                            // Emit event to frontend
                            let _ = app_handle_clone.emit("ws-message", &server_msg);
                            
                            // Emit specific events for easier handling
                            match &server_msg {
                                ServerMessage::Connected { .. } => {
                                    let _ = app_handle_clone.emit("ws-connected", &server_msg);
                                }
                                ServerMessage::UserUpdate { .. } => {
                                    let _ = app_handle_clone.emit("ws-user-update", &server_msg);
                                }
                                ServerMessage::PermissionsUpdate { .. } => {
                                    let _ = app_handle_clone.emit("ws-permissions-update", &server_msg);
                                }
                                ServerMessage::PlaylistUpdate { .. } => {
                                    let _ = app_handle_clone.emit("ws-playlist-update", &server_msg);
                                }
                                ServerMessage::ActivityLog { .. } => {
                                    let _ = app_handle_clone.emit("ws-activity-log", &server_msg);
                                }
                                ServerMessage::Pause => {
                                    let _ = app_handle_clone.emit("ws-pause", ());
                                }
                                ServerMessage::Play => {
                                    let _ = app_handle_clone.emit("ws-play", ());
                                }
                                ServerMessage::Seek { position } => {
                                    let _ = app_handle_clone.emit("ws-seek", position);
                                }
                                ServerMessage::SubtitleTrack { index } => {
                                    let _ = app_handle_clone.emit("ws-subtitle-track", index);
                                }
                                ServerMessage::AudioTrack { index } => {
                                    let _ = app_handle_clone.emit("ws-audio-track", index);
                                }
                                ServerMessage::Ready { .. } => {
                                    let _ = app_handle_clone.emit("ws-ready", &server_msg);
                                }
                                ServerMessage::AllReady => {
                                    let _ = app_handle_clone.emit("ws-all-ready", ());
                                }
                                ServerMessage::VideoUploaded { .. } => {
                                    let _ = app_handle_clone.emit("ws-video-uploaded", &server_msg);
                                }
                                ServerMessage::OwnershipTransferred { .. } => {
                                    let _ = app_handle_clone.emit("ws-ownership-transferred", &server_msg);
                                }
                                ServerMessage::Error { message } => {
                                    let _ = app_handle_clone.emit("ws-error", message);
                                }
                                ServerMessage::Pong => {
                                    println!("Heartbeat: Received pong from server");
                                },
                                ServerMessage::DownloadProgress { client_id, video_id, filename, downloaded, total, progress, speed, speed_display } => {
                                    let _ = app_handle_clone.emit("ws-download-progress", json!({
                                        "type": "download_progress",
                                        "client_id": client_id,
                                        "video_id": video_id,
                                        "filename": filename,
                                        "downloaded": downloaded,
                                        "total": total,
                                        "progress": progress,
                                        "speed": speed,
                                        "speed_display": speed_display
                                    }));
                                }
                            }
                        }
                    }
                    WsMessage::Close { code, reason } => {
                        println!("WebSocket closed. Code: {:?}, Reason: {:?}", code, reason);
                        *is_connected.write().await = false;
                        *client_id_ref.write().await = None;
                        *is_owner_ref.write().await = false;
                        let _ = app_handle_clone.emit("ws-disconnected", ());
                        break;
                    }
                    WsMessage::Ping(_) => {
                        println!("Received WebSocket ping from server");
                    }
                    WsMessage::Pong(_) => {
                        println!("Received WebSocket pong from server");
                    }
                    _ => {
                        println!("Received other WebSocket message type");
                    }
                }
            }
            Some(Err(e)) => {
                eprintln!("WebSocket receive error: {:?}", e);
                println!("Connection lost due to error, cleaning up");
                break;
            }
            None => {
                println!("WebSocket stream ended (server disconnected)");
                break;
            }
        }
    }

            // Connection closed (loop ended) - ensure cleanup
            println!("WebSocket receive loop ended, cleaning up");
            *is_connected.write().await = false;
            *client_id_ref.write().await = None;
            *is_owner_ref.write().await = false;
            let _ = app_handle_clone.emit("ws-disconnected", ());
        });

        // Spawn heartbeat task to keep connection alive (every 20 seconds)
        let sender_clone = self.sender.clone();
        let is_connected_clone = self.is_connected.clone();
        tokio::spawn(async move {
            let mut ping_count = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
                
                // Check if still connected
                if !*is_connected_clone.read().await {
                    println!("Heartbeat: Connection closed, stopping heartbeat");
                    break;
                }
                
                // Send ping message
                if let Some(sender) = sender_clone.read().await.as_ref() {
                    ping_count += 1;
                    if sender.send(ClientMessage::Ping).is_err() {
                        println!("Heartbeat: Failed to send ping, connection may be dead");
                        break;
                    }
                    println!("Heartbeat: Sent ping #{}", ping_count);
                } else {
                    println!("Heartbeat: No sender available");
                    break;
                }
            }
            println!("Heartbeat task ended");
        });

        Ok(())
    }

    pub async fn disconnect(&mut self) -> CommandResult<()> {
        *self.is_connected.write().await = false;
        *self.sender.write().await = None;
        Ok(())
    }

    pub async fn send(&self, message: ClientMessage) -> CommandResult<()> {
        let sender_guard = self.sender.read().await;
        let sender = sender_guard
            .as_ref()
            .ok_or("Not connected".to_string())?;

        sender
            .send(message)
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
}