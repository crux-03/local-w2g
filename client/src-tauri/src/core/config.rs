use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use url::Url;

const CONFIG_FILENAME: &str = "config.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub mpv_binary_path: PathBuf,
    pub videos_directory: PathBuf,
    pub username: String,
    pub server_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpv_binary_path: PathBuf::from("mpv"), // Assumes it's in PATH by default
            videos_directory: PathBuf::from(""),
            username: "Guest".to_string(),
            server_url: "http://localhost:8080".to_string(),
        }
    }
}

impl Config {
    fn path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to resolve config directory: {e}"))?;
        Ok(dir.join(CONFIG_FILENAME))
    }

    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = Self::path(app)?;

        println!("Loading config from: {:?}", path);

        if !path.exists() {
            let config = Self::default();
            config.write_to(&path)?;
            return Ok(config);
        }

        let data =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {e}"))?;

        let mut config: Self =
            serde_json::from_str(&data).map_err(|e| format!("Failed to parse config file: {e}"))?;

        // Clean and validate the data after loading
        config.sanitize_and_validate()?;

        Ok(config)
    }

    /// Performs "fixing" (sanitization) and checks for logical errors.
    fn sanitize_and_validate(&mut self) -> Result<(), String> {
        // 1. URL Robustness
        let mut raw_url = self.server_url.trim().to_string();

        // Add protocol if user forgot it (defaults to http)
        if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
            raw_url = format!("http://{}", raw_url);
        }

        let parsed = Url::parse(&raw_url).map_err(|e| format!("Invalid Server URL: {}", e))?;

        // Normalize: Remove trailing slash and update the field
        self.server_url = parsed.to_string().trim_end_matches('/').to_string();

        // 2. Path Validation (Optional but recommended)
        // Note: We don't error out if they don't exist (maybe the disk isn't mounted),
        // but we can log warnings or clean up whitespaces.
        self.username = self.username.trim().to_string();

        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = Self::path(app)?;
        self.write_to(&path)
    }

    fn write_to(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        fs::write(path, json).map_err(|e| format!("Failed to write config file: {e}"))
    }
}
