use std::{collections::HashMap, path::PathBuf, sync::Arc};

use tauri::AppHandle;
use tokio::sync::{Mutex, RwLock, broadcast::{self, error::RecvError}};

use crate::{
    CommandResult, core::{Config, FileEvent, FileManager}, protocol::{ClientMessage, Snowflake}, ws::WsHandle
};

#[derive(Clone)]
pub struct AppState {
    config: Arc<RwLock<Config>>,
    ws: Arc<RwLock<Option<WsHandle>>>,
    file_manager: Arc<RwLock<Option<Arc<FileManager>>>>,
    client_id: Arc<Mutex<Option<Snowflake>>>,
    password: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new(handle: &AppHandle) -> Result<Self, String> {
        let config = Config::load(handle)?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            ws: Arc::new(RwLock::new(None)),
            file_manager: Arc::new(RwLock::new(None)),
            client_id: Arc::new(Mutex::new(None)),
            password: Arc::new(Mutex::new(String::new())),
        })
    }

    pub async fn ws_send(&self, msg: ClientMessage) -> CommandResult<()> {
        let ws_lock = self.ws.read().await;
        let ws = ws_lock
            .as_ref()
            .ok_or("Websocket client is unbound".to_string())?;
        ws.send(msg).map_err(|e| e.to_string())
    }

    pub fn config(&self) -> &Arc<RwLock<Config>> {
        &self.config
    }

    pub async fn init_file_manager(&self, dir: PathBuf) -> Result<(), String> {
        let fm = FileManager::new(dir).map_err(|e| e.to_string())?;

        let mut events = fm.subscribe();
        let state = self.clone();
        tokio::spawn(async move {
            let mut widgets: HashMap<Snowflake, Snowflake> = HashMap::new();
            loop {
                match events.recv().await {
                    Ok(FileEvent::Started { video_id, widget_id }) => {
                        widgets.insert(video_id, widget_id);
                    }
                    Ok(FileEvent::Progress { video_id, bytes_done, .. }) => {
                        if let Some(&widget_id) = widgets.get(&video_id) {
                            let _ = state.ws_send(ClientMessage::DownloadProgress {
                                widget_id,
                                bytes_done,
                            }).await;
                        }
                    }
                    Ok(FileEvent::Completed { video_id }) => {
                        if let Some(widget_id) = widgets.remove(&video_id) {
                            let _ = state.ws_send(ClientMessage::DownloadDone { widget_id }).await;
                        }
                        let _ = state.ws_send(ClientMessage::AssertReady {
                            video_id,
                            on_device: true,
                        }).await;
                    }
                    Ok(FileEvent::Failed { video_id, .. }) => {
                        widgets.remove(&video_id);
                    }
                    Ok(FileEvent::Removed { video_id }) => {
                        let _ = state.ws_send(ClientMessage::AssertReady {
                            video_id,
                            on_device: false,
                        }).await;
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });

        *self.file_manager.write().await = Some(fm);
        Ok(())
    }

    pub async fn fm(&self) -> Result<Arc<FileManager>, String> {
        self.file_manager
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| "File manager not initialized".to_string())
    }

    pub fn client_id(&self) -> &Arc<Mutex<Option<Snowflake>>> {
        &self.client_id
    }

    pub fn password(&self) -> &Arc<Mutex<String>> {
        &self.password
    }

    pub async fn set_client_id(&self, id: Snowflake) {
        *self.client_id.lock().await = Some(id);
    }

    pub async fn set_password(&self, pw: String) {
        *self.password.lock().await = pw;
    }

    pub async fn set_ws_handle(&self, handle: WsHandle) {
        *self.ws.write().await = Some(handle)
    }

    pub async fn set_server_url(&self, app: &AppHandle, server_url: String) -> Result<(), String> {
        let mut config = self.config().write().await;
        config.server_url = server_url;
        config.save(app)
    }

    pub async fn set_username(&self, app: &AppHandle, username: String) -> Result<(), String> {
        let mut config = self.config().write().await;
        config.username = username;
        config.save(app)
    }

    pub async fn set_mpv_binary(
        &self,
        app: &AppHandle,
        mpv_binary: impl Into<PathBuf>,
    ) -> Result<(), String> {
        let mut config = self.config().write().await;
        config.mpv_binary_path = mpv_binary.into();
        config.save(app)
    }

    pub async fn set_videos_dir(
        &self,
        app: &AppHandle,
        vid_dir: impl Into<PathBuf>,
    ) -> Result<(), String> {
        let mut config = self.config().write().await;
        config.videos_directory = vid_dir.into();
        config.save(app)
    }
}
