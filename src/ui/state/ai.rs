// src/app/state/ai.rs

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc::{Receiver, Sender};

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
