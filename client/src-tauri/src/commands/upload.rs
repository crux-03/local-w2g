use tauri::State;
use tokio::io::AsyncReadExt;

use crate::{core::AppState, protocol::Snowflake, CommandResult};

const CHUNK_SIZE: usize = 50 * 1024 * 1024; // 50 MB; stays well under CF's 100 MB.

#[derive(serde::Serialize)]
struct UploadInitBody {
    filename: String,
    total_size: u64,
}

#[derive(serde::Deserialize)]
struct UploadInitResponse {
    upload_id: Snowflake,
}

async fn check_status(resp: reqwest::Response, ctx: &str) -> Result<reqwest::Response, String> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let text = resp.text().await.unwrap_or_else(|_| "Unknown error".into());
    Err(format!("{ctx} failed ({status}): {text}"))
}

#[tauri::command]
pub async fn upload_video(file_path: String, state: State<'_, AppState>) -> CommandResult<()> {
    let config = state.config().read().await;
    let raw_url = config.server_url.clone();
    drop(config);

    let server_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url
    } else {
        format!("http://{}", raw_url)
    };

    let client_id = state
        .client_id()
        .lock()
        .await
        .as_ref()
        .ok_or("Not connected")?
        .clone();
    let password = state.password().lock().await.clone();

    let mut file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let file_size = file
        .metadata()
        .await
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    let client = reqwest::Client::new();

    // 1. Init: server validates perms + extension, reserves video_id, creates .part.
    let init_url = format!("{}/api/v1/upload/init?client_id={}", server_url, client_id);
    let init_resp = client
        .post(&init_url)
        .header("X-Access-Key", &password)
        .json(&UploadInitBody {
            filename: filename.clone(),
            total_size: file_size,
        })
        .send()
        .await
        .map_err(|e| format!("Init request failed: {e}"))?;
    let init: UploadInitResponse = check_status(init_resp, "Init")
        .await?
        .json()
        .await
        .map_err(|e| format!("Init parse failed: {e}"))?;
    let upload_id = init.upload_id;

    // 2. Chunks: read sequentially, POST as raw octet-stream.
    let mut sent: u64 = 0;
    while sent < file_size {
        let remaining = (file_size - sent) as usize;
        let this_chunk = remaining.min(CHUNK_SIZE);
        let mut buf = vec![0u8; this_chunk];
        file.read_exact(&mut buf)
            .await
            .map_err(|e| format!("Read failed: {e}"))?;

        let chunk_url = format!(
            "{}/api/v1/upload/chunk?client_id={}&upload_id={}",
            server_url, client_id, upload_id
        );
        let resp = client
            .post(&chunk_url)
            .header("X-Access-Key", &password)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(buf)
            .send()
            .await
            .map_err(|e| format!("Chunk request failed: {e}"))?;
        check_status(resp, "Chunk").await?;

        sent += this_chunk as u64;
    }

    // 3. Finalize: rename + register + close widget.
    let finalize_url = format!(
        "{}/api/v1/upload/finalize?client_id={}&upload_id={}",
        server_url, client_id, upload_id
    );
    let resp = client
        .post(&finalize_url)
        .header("X-Access-Key", &password)
        .send()
        .await
        .map_err(|e| format!("Finalize request failed: {e}"))?;
    check_status(resp, "Finalize").await?;

    Ok(())
}
