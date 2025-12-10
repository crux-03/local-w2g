use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = "config.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: u64,

    #[serde(default = "default_video_dir")]
    pub video_storage_dir: String,
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_max_file_size() -> u64 {
    2048 * 1024 * 1024 // 2GB in MB
}

fn default_video_dir() -> String {
    "./videos".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            max_file_size_mb: default_max_file_size(),
            video_storage_dir: default_video_dir(),
        }
    }
}

impl Config {
    /// Load config from file, creating or fixing it if necessary
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config = if Path::new(CONFIG_PATH).exists() {
            // Load existing config
            let contents = fs::read_to_string(CONFIG_PATH)?;
            
            // Parse with defaults for missing fields
            let config: Config = serde_yaml::from_str(&contents)
                .unwrap_or_else(|e| {
                    tracing::error!("Warning: Config parse error ({}), using defaults", e);
                    Config::default()
                });
            
            config
        } else {
            // Create new config with defaults
            tracing::info!("Config file not found, creating default config.yaml");
            Config::default()
        };

        // Always write back to ensure file is complete and formatted
        config.save()?;

        // Create video storage directory if it doesn't exist
        fs::create_dir_all(&config.video_storage_dir)?;

        Ok(config)
    }

    /// Save config to file
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
        fs::write(CONFIG_PATH, yaml)?;
        Ok(())
    }
}