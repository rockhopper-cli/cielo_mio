// src/config.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    // 1. PostgreSQL Connection Settings
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_name: String,
    pub manage_db_container: bool,
    pub db_container_name: String,

    // 2. Exchange & Downloader Defaults
    pub default_output_dir: PathBuf,
    pub auto_ingest_default: bool,
    pub test_mode_default: bool,
    pub test_cap_default: usize,
    pub default_market: String, // "NSE" or "BSE"
    pub download_xml_default: bool,
    pub download_pdf_default: bool,

    // 3. Podman Container & Local Model Management
    pub manage_ollama_container: bool,
    pub ollama_container_name: String,
    pub ollama_url: String,
    pub ollama_model: String,

    // 4. Cloud LLM Settings
    pub gemini_model: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_host: "localhost".to_string(),
            db_port: 5432,
            db_user: "rockhopper_dev".to_string(),
            db_name: "brsr_db_test".to_string(),
            manage_db_container: false,
            db_container_name: "postgres".to_string(),

            default_output_dir: PathBuf::from("./downloads"),
            auto_ingest_default: true,
            test_mode_default: true,
            test_cap_default: 10,
            default_market: "NSE".to_string(),
            download_xml_default: true,
            download_pdf_default: false,

            manage_ollama_container: false,
            ollama_container_name: "qwen-coder".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "qwen2.5-coder:latest".to_string(),

            gemini_model: "gemini-flash-latest".to_string(),
        }
    }
}

impl AppConfig {
    /// Resolves ~/.config/cielo_mio/config.toml (standard XDG path)
    pub fn config_path() -> PathBuf {
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            PathBuf::from(".")
        };

        base.join("cielo_mio").join("config.toml")
    }

    /// Loads TOML from disk with backup protection against corrupted configuration files
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<AppConfig>(&content) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        eprintln!(
                            "⚠ Error parsing {}: {}. Backing up corrupted file...",
                            path.display(),
                            e
                        );
                        let backup_path = path.with_extension("toml.corrupted");
                        let _ = fs::copy(&path, &backup_path);
                        eprintln!(
                            "✓ Corrupted configuration backed up to {}",
                            backup_path.display()
                        );
                    }
                },
                Err(e) => {
                    eprintln!(
                        "⚠ Could not read configuration at {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        let default_cfg = Self::default();
        // Only auto-save default config if file does not exist (prevents wiping corrupted files)
        if !path.exists() {
            let _ = default_cfg.save();
        }
        default_cfg
    }

    /// Automatically writes TOML to ~/.config/cielo_mio/config.toml
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize TOML config: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write TOML config: {}", e))?;
        Ok(())
    }
}
