// src/system.rs

pub mod config;
pub mod container;
pub mod security;

// Convenient top-level re-exports
pub use config::AppConfig;
pub use container::{
    ContainerStatus, check_service_health, is_container_running, is_pod, start_container,
    stop_container,
};
pub use security::{KEY_DB_PASSWORD, KEY_GEMINI_API, SERVICE_NAME, SecretManager};
