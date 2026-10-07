// src/app/ai_drawer.rs

use crate::ai::agent::RagAgent;
use crate::ai::client::{LlmClient, LlmProvider};
use crate::app::state::{AppState, ChatMessage, CopilotTelemetry, SelectedLlm};
use eframe::egui::{self, Color32, Key, Margin, RichText, ScrollArea, Stroke, TextEdit, Ui};
use std::sync::atomic::Ordering;
use std::time::Instant;

pub fn render_ai_copilot_drawer(ui: &mut Ui, state: &mut AppState, contextual_hint: &str) {
    ui.horizontal(|ui| {
        let toggle_label = if state.ai.is_open {
            "🤖 Close AI Copilot ✖"
        } else {
            "🤖 Open AI Copilot ➔"
        };

        if ui
            .button(
                RichText::new(toggle_label)
                    .strong()
                    .color(Color32::from_rgb(110, 200, 255)),
            )
            .clicked()
        {
            state.ai.is_open = !state.ai.is_open;
        }

        if !state.ai.is_open {
            ui.label(
                RichText::new(format!("Copilot ready: {}", contextual_hint))
                    .small()
                    .color(Color32::from_rgb(150, 160, 175)),
            );
        }
    });

    if !state.ai.is_open {
        return;
    }

    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(18, 22, 30))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 55, 75)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🤖 ESG Copilot")
                        .strong()
                        .color(Color32::from_rgb(90, 210, 140)),
                );

                ui.separator();

                ui.radio_value(&mut state.ai.selected_llm, SelectedLlm::Gemini, "Gemini API");
                ui.radio_value(&mut state.ai.selected_llm, SelectedLlm::Ollama, "Local Ollama");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🗑 Clear").clicked() {
                        state.ai.chat_history.clear();
                    }
                });
            });

            ui.add_space(4.0);

            // Active Context Indicator
            ui.label(
                RichText::new(format!("Active Context: {}", contextual_hint))
                    .small()
                    .italics()
                    .color(Color32::from_rgb(130, 170, 220)),
            );

            // API Diagnostics Inspection Banner
            if let Some(ref telem) = state.ai.last_telemetry {
                ui.add_space(2.0);
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "⚡ API Telemetry: {} | Latency: {}ms | Status: {}",
                        telem.provider,
                        telem.latency_ms,
                        telem.http_status.map_or("Error".to_string(), |s| s.to_string())
                    ))
                    .small()
                    .color(if telem.error.is_none() {
                        Color32::from_rgb(100, 210, 140)
                    } else {
                        Color32::from_rgb(240, 100, 100)
                    }),
                )
                .id_salt("copilot_telemetry_collapsing")
                .show(ui, |ui| {
                    ui.monospace(format!("Timestamp: {}", telem.timestamp));
                    ui.monospace(format!("Model: {}", telem.model));
                    ui.monospace(format!("Endpoint: {}", telem.endpoint));
                    ui.monospace(format!("Facts Injected: {} verified records", telem.facts_injected));
                    ui.monospace(format!("Payload Size: {} chars (Prompt) ➔ {} chars (Response)", telem.prompt_chars, telem.response_chars));
                    if let Some(ref err) = telem.error {
                        ui.label(RichText::new(format!("Error: {}", err)).color(Color32::from_rgb(240, 90, 90)).small());
                    }
                });
            }

            ui.add_space(4.0);

            // Chat Messages
            ScrollArea::vertical()
                .max_height(220.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &state.ai.chat_history {
                        ui.add_space(3.0);
                        if msg.role == "user" {
                            ui.label(
                                RichText::new(format!("👤 You: {}", msg.content))
                                    .small()
                                    .color(Color32::from_rgb(120, 190, 255)),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("🌿 Copilot: {}", msg.content))
                                    .small()
                                    .color(Color32::from_rgb(220, 230, 240)),
                            );
                        }
                    }

                    if state.ai.is_thinking.load(Ordering::Relaxed) {
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Querying verified database repository...").small().color(Color32::GRAY));
                        });
                    }
                });

            ui.add_space(6.0);

            // Suggested Prompts
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Suggestions:").small().color(Color32::GRAY));
                if ui.small_button("Scope 1 & 2 Emissions?").clicked() {
                    state.ai.current_prompt = "What are the Scope 1 and Scope 2 GHG emissions disclosed, and how do they compare with prior year?".to_string();
                }
                if ui.small_button("Water & Waste Disclosures?").clicked() {
                    state.ai.current_prompt = "Summarize total water withdrawal and waste generated as reported in the filing.".to_string();
                }
                if ui.small_button("Gender Diversity & Wages?").clicked() {
                    state.ai.current_prompt = "What percentage of board directors and workforce are women, and what are gross wages to females?".to_string();
                }
                if ui.small_button("POSH Complaints?").clicked() {
                    state.ai.current_prompt = "How many POSH complaints were filed, pending, and upheld during the financial year?".to_string();
                }
            });

            ui.add_space(4.0);

            // Prompt Input
            ui.horizontal(|ui| {
                let is_busy = state.ai.is_thinking.load(Ordering::Relaxed);
                let text_resp = ui.add_enabled(
                    !is_busy,
                    TextEdit::singleline(&mut state.ai.current_prompt)
                        .hint_text("Ask about specific ESG disclosures...")
                        .desired_width(ui.available_width() - 80.0),
                );

                let send_clicked = ui.add_enabled(!is_busy, egui::Button::new(RichText::new("Send ➔").strong())).clicked();
                let enter_hit = text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));

                if (send_clicked || enter_hit) && !state.ai.current_prompt.trim().is_empty() && !is_busy {
                    let prompt = state.ai.current_prompt.trim().to_string();
                    state.ai.current_prompt.clear();

                    state.ai.chat_history.push(ChatMessage {
                        role: "user",
                        content: prompt.clone(),
                    });

                    let (grounded_context, fact_count, company_name, cin) = build_grounded_context(state);

                    state.add_log(format!("[COPILOT] Query initiated for '{}': \"{}\"", company_name, prompt));
                    dispatch_copilot_request(state, prompt, grounded_context, fact_count, company_name, cin);
                }
            });
        });

    ui.add_space(6.0);
}

/// Builds verified context from active state to eliminate hallucinations
fn build_grounded_context(state: &AppState) -> (String, usize, String, String) {
    let mut lines = Vec::new();

    if state.active_tab == crate::app::state::ActiveTab::FormViewer {
        let active_cin = state.form_viewer.selected_cin.clone();
        let company_name = state
            .form_viewer
            .available_filings
            .iter()
            .find(|f| f.corporate_id_number == active_cin)
            .map(|f| f.company_name.clone())
            .unwrap_or_else(|| "Active Entity".to_string());

        let mut count = 0;
        for fact in &state.form_viewer.current_facts {
            if !fact.raw_value.is_empty() {
                lines.push(format!(
                    " - [{}] {} ({}) = {} {}",
                    fact.section,
                    fact.standard_indicator,
                    fact.context_ref,
                    fact.raw_value,
                    fact.standard_unit.as_deref().unwrap_or("")
                ));
                count += 1;
                if count >= 60 {
                    break;
                }
            }
        }
        (lines.join("\n"), count, company_name, active_cin)
    } else {
        let active_cin = state.analytics.selected_cin.clone();
        let matching: Vec<&crate::app::state::CompanyEsgMetrics> = state
            .analytics
            .metrics
            .iter()
            .filter(|m| m.corporate_id_number == active_cin)
            .collect();

        if let Some(latest) = matching.last() {
            lines.push(format!("INDUSTRY SECTOR: {}", latest.industry_sector));
            lines.push(format!(
                " - Scope 1 GHG: {:.1} tCO2e",
                latest.scope_1_tco2e.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Scope 2 GHG: {:.1} tCO2e",
                latest.scope_2_tco2e.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Scope 3 GHG: {:.1} tCO2e",
                latest.scope_3_tco2e.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Renewable Energy: {:.1}%",
                latest.renewable_energy_pct.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Water Withdrawal: {:.0} KL",
                latest.water_withdrawal_kl.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Female Board Directors: {:.1}%",
                latest.female_board_directors_pct.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Female Workforce: {:.1}%",
                latest.female_workforce_pct.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Gross Wages to Female: {:.1}%",
                latest.gross_wages_to_female_pct.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Employee Turnover Rate: {:.1}%",
                latest.employee_turnover_pct.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Fatalities: {:.0}",
                latest.fatalities.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Work-Related Injuries: {:.0}",
                latest.work_related_injuries.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - POSH Complaints Upheld: {:.0}",
                latest.posh_complaints_upheld.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Total POSH Complaints: {:.0}",
                latest.total_posh_complaints.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Complaints Filed: {:.0}",
                latest.complaints_filed.unwrap_or(0.0)
            ));
            lines.push(format!(
                " - Pending Complaints: {:.0}",
                latest.pending_complaints.unwrap_or(0.0)
            ));
            (
                lines.join("\n"),
                16,
                latest.company_name.clone(),
                active_cin,
            )
        } else {
            (
                "No entity selected in active dashboard.".to_string(),
                0,
                "General Entity".to_string(),
                String::new(),
            )
        }
    }
}

fn dispatch_copilot_request(
    state: &mut AppState,
    user_query: String,
    grounded_context: String,
    fact_count: usize,
    company_name: String,
    cin: String,
) {
    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            state.ai.chat_history.push(ChatMessage {
                role: "assistant",
                content: "Database disconnected. Please reconnect your database in Settings."
                    .to_string(),
            });
            return;
        }
    };

    // Telemetry reflects the actual /api/chat endpoint
    let (provider_label, model_label, endpoint_url) = match state.ai.selected_llm {
        SelectedLlm::Gemini => (
            "Google Gemini API".to_string(),
            state.ai.gemini_model.clone(),
            "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
        ),
        SelectedLlm::Ollama => (
            "Local Ollama".to_string(),
            state.ai.ollama_model.clone(),
            format!("{}/api/chat", state.ai.ollama_url.trim_end_matches('/')),
        ),
    };

    state.add_log(format!(
        "[COPILOT] Querying {} ({}) for '{}' ({} context facts injected)",
        provider_label, model_label, company_name, fact_count
    ));

    let provider = match state.ai.selected_llm {
        SelectedLlm::Gemini => {
            if state.ai.gemini_key.trim().is_empty() {
                state.add_log("[COPILOT ERROR] Missing Gemini API Key in Settings.".to_string());
                state.ai.chat_history.push(ChatMessage {
                    role: "assistant",
                    content: "Missing Gemini API Key. Please add it in the Settings tab."
                        .to_string(),
                });
                return;
            }
            LlmProvider::Gemini {
                model: state.ai.gemini_model.clone(),
                api_key: state.ai.gemini_key.trim().to_string(),
            }
        }
        SelectedLlm::Ollama => LlmProvider::Ollama {
            model: state.ai.ollama_model.clone(),
            endpoint: state.ai.ollama_url.clone(),
        },
    };

    let is_thinking = state.ai.is_thinking.clone();
    is_thinking.store(true, Ordering::Relaxed);
    let chat_tx = state.ai.chat_tx.clone();
    let telem_tx = state.ai.telemetry_tx.clone();
    let event_tx = state.event_tx.clone();

    let approx_prompt_chars = user_query.len() + grounded_context.len() + 400;
    let start_time = Instant::now();

    tokio::spawn(async move {
        let client = LlmClient::new(provider);
        let agent = RagAgent::new(pool, client);

        let result = agent
            .answer_question(&company_name, &cin, &grounded_context, &user_query)
            .await;

        let latency_ms = start_time.elapsed().as_millis();
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

        match result {
            Ok(ans) => {
                let resp_chars = ans.len();
                let _ = event_tx
                    .send(crate::downloader::types::DownloadEvent::Log(format!(
                        "[COPILOT] HTTP 200 OK | Latency: {}ms | Response: {} chars",
                        latency_ms, resp_chars
                    )))
                    .await;

                let _ = telem_tx
                    .send(CopilotTelemetry {
                        timestamp,
                        provider: provider_label,
                        model: model_label,
                        endpoint: endpoint_url,
                        http_status: Some(200),
                        latency_ms,
                        facts_injected: fact_count,
                        prompt_chars: approx_prompt_chars,
                        response_chars: resp_chars,
                        error: None,
                    })
                    .await;

                let _ = chat_tx
                    .send(ChatMessage {
                        role: "assistant",
                        content: ans,
                    })
                    .await;
            }
            Err(e) => {
                let err_str = e.to_string();
                let _ = event_tx
                    .send(crate::downloader::types::DownloadEvent::Log(format!(
                        "[COPILOT ERROR] Request failed after {}ms: {}",
                        latency_ms, err_str
                    )))
                    .await;

                let _ = telem_tx
                    .send(CopilotTelemetry {
                        timestamp,
                        provider: provider_label,
                        model: model_label,
                        endpoint: endpoint_url,
                        http_status: None,
                        latency_ms,
                        facts_injected: fact_count,
                        prompt_chars: approx_prompt_chars,
                        response_chars: 0,
                        error: Some(err_str.clone()),
                    })
                    .await;

                let _ = chat_tx
                    .send(ChatMessage {
                        role: "assistant",
                        content: format!("Error from AI Copilot: {}", err_str),
                    })
                    .await;
            }
        }

        is_thinking.store(false, Ordering::Relaxed);
    });
}
