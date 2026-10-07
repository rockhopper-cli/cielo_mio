// src/app/state.rs

use crate::config::AppConfig;
use crate::container::ContainerStatus;
use crate::downloader::types::{DownloadEvent, Market};

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Download,
    FormViewer,
    Analytics,
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedLlm {
    Gemini,
    Ollama,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct CopilotTelemetry {
    pub timestamp: String,
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub http_status: Option<u16>,
    pub latency_ms: u128,
    pub facts_injected: usize,
    pub prompt_chars: usize,
    pub response_chars: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompanyFilingSummary {
    pub entity_id: Uuid,
    pub corporate_id_number: String,
    pub company_name: String,
    pub address_registered_office: Option<String>,
    pub address_corporate_office: Option<String>,
    pub company_email: Option<String>,
    pub company_telephone: Option<String>,
    pub website: Option<String>,
    pub paid_up_shares_value: Option<Decimal>,
    pub fy_start: Option<NaiveDate>,
    pub fy_end: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FormFactRow {
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub standard_indicator: String,
    pub context_ref: String,
    pub raw_value: String,
    pub standard_numeric_value: Option<Decimal>,
    pub standard_unit: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FormMasterItem {
    pub question_key: String,
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub question_in_form: Option<i16>,
    pub question_number: i16,
    pub question_order: i16,
    pub response_datatype: String,
    pub field_flag: i16,
    pub append_to_field: Option<String>,
    pub section_text: String,
    pub principle_text: String,
    pub sub_section_text: String,
    pub indicator_text: String,
    pub question_text: String,
    pub standard_indicator_capmkt: String,
    pub standard_context_reference: String,
    pub input_text: Option<String>,
}

#[derive(Debug)]
pub enum FormViewerEvent {
    FilingsLoaded(Vec<CompanyFilingSummary>),
    TemplateLoaded(Vec<FormMasterItem>),
    FactsLoaded {
        entity_id: Uuid,
        facts: Vec<FormFactRow>,
    },
    Error(String),
}

pub struct FormViewerState {
    pub available_filings: Vec<CompanyFilingSummary>,
    pub unique_companies: Vec<(String, String)>,
    pub selected_cin: String,
    pub selected_entity_id: Option<Uuid>,
    pub filings_loaded: bool,
    pub company_search: String,

    pub selected_section: String,
    pub selected_sub_section: i16,
    pub selected_principle: i16,
    pub selected_indicator_type: Option<String>,

    pub form_template: Vec<FormMasterItem>,
    pub template_loaded: bool,

    pub current_facts: Vec<FormFactRow>,
    pub cached_entity_id: Option<Uuid>,
    pub is_loading_facts: bool,

    pub search_query: String,
}

impl Default for FormViewerState {
    fn default() -> Self {
        Self {
            available_filings: Vec::new(),
            unique_companies: Vec::new(),
            selected_cin: String::new(),
            selected_entity_id: None,
            filings_loaded: false,
            company_search: String::new(),
            selected_section: "A".to_string(),
            selected_sub_section: 0,
            selected_principle: 0,
            selected_indicator_type: None,
            form_template: Vec::new(),
            template_loaded: false,
            current_facts: Vec::new(),
            cached_entity_id: None,
            is_loading_facts: false,
            search_query: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EsgPillar {
    Environmental,
    Social,
    Governance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentalMetric {
    Scope1Emissions,
    Scope2Emissions,
    Scope3Emissions,
    RenewableEnergy,
    WaterWithdrawal,
    EnergyIntensity,
    WasteIntensity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialMetric {
    FemaleBoardDirectors,
    GrossWagesToFemale,
    EmployeeTurnoverRate,
    WorkRelatedInjuries,
    Fatalities,
    FemaleWorkforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceMetric {
    PoshComplaintsUpheld,
    TotalPoshComplaints,
    ComplaintsFiled,
    PendingComplaints,
    DisciplinaryActions,
    CostOnWellBeing,
}

#[derive(Debug, Clone, Default)]
pub struct CompanyEsgMetrics {
    pub entity_id: Uuid,
    pub company_name: String,
    pub corporate_id_number: String,
    pub industry_sector: String,
    pub industry_division: String,
    pub fy_start: Option<NaiveDate>,
    pub fy_end: Option<NaiveDate>,
    pub fy_label: String,

    pub scope_1_tco2e: Option<f64>,
    pub scope_2_tco2e: Option<f64>,
    pub scope_3_tco2e: Option<f64>,
    pub water_withdrawal_kl: Option<f64>,
    pub energy_intensity: Option<f64>,
    pub waste_intensity: Option<f64>,

    pub female_board_directors_pct: Option<f64>,
    pub gross_wages_to_female_pct: Option<f64>,
    pub employee_turnover_pct: Option<f64>,
    pub work_related_injuries: Option<f64>,
    pub fatalities: Option<f64>,
    pub female_workforce_pct: Option<f64>,

    pub posh_complaints_upheld: Option<f64>,
    pub total_posh_complaints: Option<f64>,
    pub complaints_filed: Option<f64>,
    pub pending_complaints: Option<f64>,
    pub disciplinary_actions: Option<f64>,
    pub cost_on_wellbeing_pct: Option<f64>,

    pub paid_up_capital_cr: Option<f64>,
    pub renewable_energy_pct: Option<f64>,
}

pub struct AnalyticsState {
    pub metrics: Vec<CompanyEsgMetrics>,
    pub unique_companies: Vec<(String, String)>,
    pub is_loading: bool,
    pub data_loaded: bool,
    pub selected_pillar: EsgPillar,
    pub selected_env_metric: EnvironmentalMetric,
    pub selected_soc_metric: SocialMetric,
    pub selected_gov_metric: GovernanceMetric,
    pub selected_cin: String,
    pub selected_sector: String,
    pub search_query: String,
    pub company_search: String,
}

impl Default for AnalyticsState {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
            unique_companies: Vec::new(),
            is_loading: false,
            data_loaded: false,
            selected_pillar: EsgPillar::Environmental,
            selected_env_metric: EnvironmentalMetric::Scope1Emissions,
            selected_soc_metric: SocialMetric::FemaleWorkforce,
            selected_gov_metric: GovernanceMetric::TotalPoshComplaints,
            selected_cin: String::new(),
            selected_sector: "All Sectors".to_string(),
            search_query: String::new(),
            company_search: String::new(),
        }
    }
}

pub struct EmbeddedAiState {
    pub is_open: bool,
    pub selected_llm: SelectedLlm,
    pub gemini_key: String,
    pub gemini_model: String,
    pub ollama_model: String,
    pub ollama_url: String,
    pub chat_history: Vec<ChatMessage>,
    pub current_prompt: String,
    pub is_thinking: Arc<AtomicBool>,
    pub chat_rx: Option<Receiver<ChatMessage>>,
    pub chat_tx: Sender<ChatMessage>,
    pub telemetry_rx: Option<Receiver<CopilotTelemetry>>,
    pub telemetry_tx: Sender<CopilotTelemetry>,
    pub last_telemetry: Option<CopilotTelemetry>,
}

pub struct AppState {
    pub active_tab: ActiveTab,
    pub pool: Option<PgPool>,
    pub config: AppConfig,
    pub nav_collapsed: bool,

    pub is_reconnecting_db: Arc<AtomicBool>,
    pub db_rx: Option<Receiver<Result<PgPool, String>>>,
    pub db_tx: Sender<Result<PgPool, String>>,

    // Background-polled database container status (avoids blocking per-frame CLI inspect calls)
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
    pub analytics_rx: Option<Receiver<Vec<CompanyEsgMetrics>>>,
    pub analytics_tx: Sender<Vec<CompanyEsgMetrics>>,

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
        let (container_tx, container_rx) = tokio::sync::mpsc::channel(16);
        let (db_tx, db_rx) = tokio::sync::mpsc::channel(8);

        let gemini_key = crate::security::SecretManager::get_secret(
            crate::security::KEY_GEMINI_API,
            "GEMINI_API_KEY",
        )
        .unwrap_or_default();

        let db_password = crate::security::SecretManager::get_secret(
            crate::security::KEY_DB_PASSWORD,
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
            download_xml: true,
            download_pdf: false,
            test_mode: true,
            test_cap: 10,
            output_dir: PathBuf::from("./downloads"),
            auto_ingest: true,

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

            ai: EmbeddedAiState {
                is_open: false,
                selected_llm,
                gemini_key,
                gemini_model: "gemini-flash-latest".to_string(),
                ollama_model: "qwen2.5-coder:latest".to_string(),
                ollama_url: "http://localhost:11434".to_string(),
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

    /// O(1) push and pop using VecDeque ring buffer
    pub fn add_log(&mut self, msg: String) {
        if self.log_messages.len() >= 500 {
            self.log_messages.pop_front();
        }
        let timestamp = chrono::Local::now().format("[%H:%M:%S]").to_string();
        self.log_messages
            .push_back(format!("{} {}", timestamp, msg));
    }
}

/// Spawns a non-blocking asynchronous check for Podman database status
pub fn trigger_db_container_check(state: &AppState) {
    let name = state.config.db_container_name.clone();
    let running_flag = state.db_container_running.clone();
    tokio::task::spawn_blocking(move || {
        let running = crate::container::is_container_running(&name);
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
        let res = crate::downloader::db::connect_to_database(
            &host,
            port,
            &user,
            &password,
            &db_name,
            std::time::Duration::from_secs(3),
        )
        .await;
        let _ = tx.send(res).await;
    });
}
