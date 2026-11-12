// src/ui/state.rs

pub mod ai;
pub mod analytics;
pub mod export;
pub mod form_viewer;
pub mod geo;

pub use ai::*;
pub use analytics::*;
pub use export::*;
pub use form_viewer::*;
pub use geo::*;

use crate::domain::download::{DownloadEvent, Market};
use crate::system::{AppConfig, ContainerStatus};

use sqlx::PgPool;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Download,
    FormViewer,
    Analytics,
    GeoReports,
    Export,
    Config,
    About,
}

pub struct AppState {
    pub active_tab: ActiveTab,
    pub pool: Option<PgPool>,
    pub config: AppConfig,
    pub nav_collapsed: bool,

    pub is_reconnecting_db: Arc<AtomicBool>,
    pub db_rx: Option<Receiver<Result<PgPool, String>>>,
    pub db_tx: Sender<Result<PgPool, String>>,
    pub db_container_running: Arc<AtomicBool>,

    #[allow(dead_code)]
    pub selected_market: Market,
    pub download_xml: bool,
    pub download_pdf: bool,
    pub test_mode: bool,
    pub test_cap: usize,
    pub output_dir: PathBuf,
    pub auto_ingest: bool,

    pub is_downloading: Arc<AtomicBool>,
    pub is_ingesting: Arc<AtomicBool>,
    pub cancel_token: Option<CancellationToken>,

    pub show_logs: bool,
    pub log_messages: VecDeque<String>,
    pub event_rx: Option<Receiver<DownloadEvent>>,
    pub event_tx: Sender<DownloadEvent>,

    pub form_viewer: FormViewerState,
    pub form_rx: Option<Receiver<FormViewerEvent>>,
    pub form_tx: Sender<FormViewerEvent>,

    pub analytics: AnalyticsState,
    pub analytics_rx: Option<Receiver<Result<Vec<CompanyEsgMetrics>, String>>>,
    pub analytics_tx: Sender<Result<Vec<CompanyEsgMetrics>, String>>,

    pub geo_state: GeoReportsState,
    pub geo_rx: Option<Receiver<Result<Vec<GeoEntityMarker>, String>>>,
    pub geo_tx: Sender<Result<Vec<GeoEntityMarker>, String>>,

    pub export_state: ExportState,
    pub export_rx: Option<Receiver<(String, bool)>>,
    pub export_tx: Sender<(String, bool)>,

    pub ai: EmbeddedAiState,

    pub db_password_input: String,
    pub security_feedback_msg: Option<(String, bool)>,
    pub container_status: ContainerStatus,
    pub container_rx: Option<Receiver<ContainerStatus>>,
    pub container_tx: Sender<ContainerStatus>,
}

impl AppState {
    pub fn new(pool: Option<PgPool>, config: AppConfig) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let (chat_tx, chat_rx) = tokio::sync::mpsc::channel(32);
        let (telem_tx, telem_rx) = tokio::sync::mpsc::channel(16);
        let (form_tx, form_rx) = tokio::sync::mpsc::channel(32);
        let (analytics_tx, analytics_rx) = tokio::sync::mpsc::channel(16);
        let (geo_tx, geo_rx) = tokio::sync::mpsc::channel(16);
        let (export_tx, export_rx) = tokio::sync::mpsc::channel(16);
        let (container_tx, container_rx) = tokio::sync::mpsc::channel(16);
        let (db_tx, db_rx) = tokio::sync::mpsc::channel(8);

        let gemini_key = crate::system::security::SecretManager::get_secret(
            crate::system::security::KEY_GEMINI_API,
            "GEMINI_API_KEY",
        )
        .unwrap_or_default();

        let db_password = crate::system::security::SecretManager::get_secret(
            crate::system::security::KEY_DB_PASSWORD,
            "DB_PASSWORD",
        )
        .unwrap_or_default();

        let selected_llm = if !gemini_key.is_empty() {
            SelectedLlm::Gemini
        } else {
            SelectedLlm::Ollama
        };

        let selected_market = if config.default_market == "BSE" {
            Market::Bse
        } else {
            Market::Nse
        };

        let download_xml = config.download_xml_default;
        let download_pdf = config.download_pdf_default;
        let test_mode = config.test_mode_default;
        let test_cap = config.test_cap_default.max(2);
        let output_dir = config.default_output_dir.clone();
        let auto_ingest = config.auto_ingest_default;
        let gemini_model = config.gemini_model.clone();
        let ollama_model = config.ollama_model.clone();
        let ollama_url = config.ollama_url.clone();

        Self {
            active_tab: ActiveTab::Download,
            pool,
            config,
            nav_collapsed: false,

            is_reconnecting_db: Arc::new(AtomicBool::new(false)),
            db_rx: Some(db_rx),
            db_tx,
            db_container_running: Arc::new(AtomicBool::new(false)),

            selected_market,
            download_xml,
            download_pdf,
            test_mode,
            test_cap,
            output_dir,
            auto_ingest,

            is_downloading: Arc::new(AtomicBool::new(false)),
            is_ingesting: Arc::new(AtomicBool::new(false)),
            cancel_token: None,

            show_logs: false,
            log_messages: VecDeque::with_capacity(512),
            event_rx: Some(rx),
            event_tx: tx,

            form_viewer: FormViewerState::default(),
            form_rx: Some(form_rx),
            form_tx,

            analytics: AnalyticsState::default(),
            analytics_rx: Some(analytics_rx),
            analytics_tx,

            geo_state: GeoReportsState::default(),
            geo_rx: Some(geo_rx),
            geo_tx,

            export_state: ExportState::default(),
            export_rx: Some(export_rx),
            export_tx,

            ai: EmbeddedAiState {
                is_open: false,
                selected_llm,
                gemini_key,
                gemini_model,
                ollama_model,
                ollama_url,
                chat_history: vec![ChatMessage {
                    role: "assistant",
                    content: "Hello! I am your BRSR AI analyst. I inspect verified statutory disclosures from your active repository.".to_string(),
                }],
                current_prompt: String::new(),
                is_thinking: Arc::new(AtomicBool::new(false)),
                chat_rx: Some(chat_rx),
                chat_tx,
                telemetry_rx: Some(telem_rx),
                telemetry_tx: telem_tx,
                last_telemetry: None,
            },

            db_password_input: db_password,
            security_feedback_msg: None,

            container_status: ContainerStatus::default(),
            container_rx: Some(container_rx),
            container_tx,
        }
    }

    pub fn add_log(&mut self, msg: String) {
        if self.log_messages.len() >= 500 {
            self.log_messages.pop_front();
        }
        let timestamp = chrono::Local::now().format("[%H:%M:%S]").to_string();
        self.log_messages
            .push_back(format!("{} {}", timestamp, msg));
    }
}

pub fn trigger_db_container_check(state: &AppState) {
    let name = state.config.db_container_name.clone();
    let running_flag = state.db_container_running.clone();
    tokio::task::spawn_blocking(move || {
        let running = crate::system::container::is_container_running(&name);
        running_flag.store(running, Ordering::Relaxed);
    });
}

pub fn trigger_db_reconnect(state: &mut AppState) {
    if state.is_reconnecting_db.load(Ordering::Relaxed) {
        return;
    }
    state.is_reconnecting_db.store(true, Ordering::Relaxed);
    state.add_log(format!(
        "[INFO] Connecting to PostgreSQL at {}:{}...",
        state.config.db_host, state.config.db_port
    ));

    let host = state.config.db_host.clone();
    let port = state.config.db_port;
    let user = state.config.db_user.clone();
    let db_name = state.config.db_name.clone();
    let password = state.db_password_input.clone();
    let tx = state.db_tx.clone();

    tokio::spawn(async move {
        let res = crate::storage::connect_to_database(
            &host,
            port,
            &user,
            &password,
            &db_name,
            std::time::Duration::from_secs(3),
        )
        .await;

        if let Ok(ref pool) = res {
            let _ =
                crate::storage::repository::entities::reclassify_unclassified_entities(pool).await;
        }
        let _ = tx.send(res).await;
    });
}

/// Starts the DB container if stopped, waits briefly for Postgres readiness, and connects without blocking the UI thread
pub fn trigger_db_start_and_connect(state: &mut AppState) {
    if state.is_reconnecting_db.load(Ordering::Relaxed) {
        return;
    }
    state.is_reconnecting_db.store(true, Ordering::Relaxed);
    let c_name = state.config.db_container_name.clone();
    state.add_log(format!(
        "[INFO] Ensuring container '{}' is running and connecting to PostgreSQL...",
        c_name
    ));

    let host = state.config.db_host.clone();
    let port = state.config.db_port;
    let user = state.config.db_user.clone();
    let db_name = state.config.db_name.clone();
    let password = state.db_password_input.clone();
    let tx = state.db_tx.clone();
    let running_flag = state.db_container_running.clone();

    tokio::spawn(async move {
        let cname_clone = c_name.clone();
        let was_started = tokio::task::spawn_blocking(move || {
            if !crate::system::container::is_container_running(&cname_clone) {
                let res = crate::system::container::start_container(&cname_clone);
                running_flag.store(res.is_ok(), Ordering::Relaxed);
                res.is_ok()
            } else {
                running_flag.store(true, Ordering::Relaxed);
                false
            }
        })
        .await
        .unwrap_or(false);

        let attempts = if was_started { 5 } else { 1 };
        let mut last_res = Err("Connection failed".to_string());
        for i in 1..=attempts {
            last_res = crate::storage::connect_to_database(
                &host,
                port,
                &user,
                &password,
                &db_name,
                std::time::Duration::from_secs(3),
            )
            .await;
            if last_res.is_ok() {
                break;
            }
            if i < attempts {
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            }
        }

        if let Ok(ref pool) = last_res {
            let _ =
                crate::storage::repository::entities::reclassify_unclassified_entities(pool).await;
        }
        let _ = tx.send(last_res).await;
    });
}
