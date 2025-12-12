use crate::types::CommandResult;
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

#[cfg(not(target_os = "windows"))]
use tokio::net::UnixStream;

#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe::NamedPipeClient;

// Platform-specific stream type
#[cfg(not(target_os = "windows"))]
type IpcStream = UnixStream;

#[cfg(target_os = "windows")]
type IpcStream = NamedPipeClient;

pub struct MpvManager {
    process: Arc<RwLock<Option<Child>>>,
    socket_path: String,
    socket: Arc<RwLock<Option<IpcStream>>>,
}

impl MpvManager {
    pub fn new() -> Self {
        Self::with_socket_name("mpv-socket")
    }

    pub fn with_socket_name(name: &str) -> Self {
        #[cfg(not(target_os = "windows"))]
        let socket_path = std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .into_owned();

        #[cfg(target_os = "windows")]
        let socket_path = format!(r"\\.\pipe\{}", name);

        Self {
            process: Arc::new(RwLock::new(None)),
            socket_path,
            socket: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&mut self, mpv_path: &str, video_path: &str) -> CommandResult<()> {
        // Kill existing process if any
        self.stop().await?;

        // Start mpv with IPC enabled
        let child = Command::new(mpv_path)
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--idle=yes")
            .arg("--pause")
            .arg("--force-window=yes")
            .arg(video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start mpv: {}", e))?;

        *self.process.write().await = Some(child);

        // Wait for the socket/pipe to be created, with retries
        self.connect_with_retry(10, 100).await?;

        Ok(())
    }

    async fn connect_with_retry(&mut self, max_attempts: u32, delay_ms: u64) -> CommandResult<()> {
        let mut last_error = String::new();

        for _ in 0..max_attempts {
            match self.connect_socket().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = e;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(format!(
            "Failed to connect after {} attempts: {}",
            max_attempts, last_error
        ))
    }

    #[cfg(not(target_os = "windows"))]
    async fn connect_socket(&mut self) -> CommandResult<()> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| format!("Failed to connect to mpv socket: {}", e))?;
        *self.socket.write().await = Some(stream);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn connect_socket(&mut self) -> CommandResult<()> {
        use tokio::net::windows::named_pipe::ClientOptions;

        let client = ClientOptions::new()
            .open(&self.socket_path)
            .map_err(|e| format!("Failed to connect to mpv pipe: {}", e))?;

        *self.socket.write().await = Some(client);
        Ok(())
    }

    pub async fn stop(&mut self) -> CommandResult<()> {
        // Try to send quit command first for graceful shutdown
        if self.socket.read().await.is_some() {
            let _ = self.send_command(vec![json!("quit")]).await;
        }

        if let Some(mut process) = self.process.write().await.take() {
            let _ = process.kill().await;
        }

        *self.socket.write().await = None;

        // Clean up socket file on Unix
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        Ok(())
    }

    pub async fn send_command(&self, command: Vec<Value>) -> CommandResult<()> {
        let cmd = json!({ "command": command });
        let cmd_str = format!("{}\n", serde_json::to_string(&cmd).unwrap());

        let mut socket_guard = self.socket.write().await;
        let socket = socket_guard
            .as_mut()
            .ok_or_else(|| "Not connected to mpv".to_string())?;

        socket
            .write_all(cmd_str.as_bytes())
            .await
            .map_err(|e| format!("Failed to send command: {}", e))?;

        Ok(())
    }

    pub async fn pause(&self) -> CommandResult<()> {
        self.send_command(vec![json!("set_property"), json!("pause"), json!(true)])
            .await
    }

    pub async fn play(&self) -> CommandResult<()> {
        self.send_command(vec![json!("set_property"), json!("pause"), json!(false)])
            .await
    }

    pub async fn seek(&self, position: f64) -> CommandResult<()> {
        self.send_command(vec![json!("seek"), json!(position), json!("absolute")])
            .await
    }

    pub async fn set_subtitle_track(&self, index: u32) -> CommandResult<()> {
        // In mpv, subtitle tracks are indexed starting from 1
        // index 0 typically means "no subtitles"
        self.send_command(vec![json!("set_property"), json!("sid"), json!(index)])
            .await
    }

    pub async fn set_audio_track(&self, index: u32) -> CommandResult<()> {
        // In mpv, audio tracks are indexed starting from 1
        self.send_command(vec![json!("set_property"), json!("aid"), json!(index)])
            .await
    }

    pub async fn _get_property(&self, property: &str) -> CommandResult<Value> {
        // Note: This is a simplified version. For proper property getting,
        // you'd need to implement response parsing from mpv's JSON IPC.
        self.send_command(vec![json!("get_property"), json!(property)])
            .await?;
        Ok(json!(null))
    }

    pub async fn is_running(&self) -> bool {
        self.process.read().await.is_some()
    }
}
