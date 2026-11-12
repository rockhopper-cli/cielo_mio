// src/ui/events.rs

use super::state::{AppState, FormViewerEvent};
use std::sync::atomic::Ordering;

/// Drains all incoming asynchronous background messages from Tokio channels into AppState
pub fn drain_incoming_events(state: &mut AppState) {
    // 1. Ingestion & Download Activity Logs
    if let Some(ref mut rx) = state.event_rx {
        let mut new_logs = Vec::new();
        while let Ok(event) = rx.try_recv() {
            match event {
                crate::domain::download::DownloadEvent::Log(msg) => {
                    if msg.contains("Ingestion Complete") {
                        state.form_viewer.filings_loaded = false;
                        state.analytics.data_loaded = false;
                        state.geo_state.markers.clear();
                    }
                    new_logs.push(format!("[LOG] {}", msg));
                }
                crate::domain::download::DownloadEvent::FileDownloaded { file_name, path } => {
                    new_logs.push(format!("[SAVED] {} -> {}", file_name, path.display()));
                }
                crate::domain::download::DownloadEvent::FileSkipped { file_name, reason } => {
                    new_logs.push(format!("[SKIPPED] {}: {}", file_name, reason));
                }
                crate::domain::download::DownloadEvent::FileFailed { file_name, error } => {
                    new_logs.push(format!("[FAILED] {}: {}", file_name, error));
                }
                crate::domain::download::DownloadEvent::Done {
                    total_downloaded,
                    total_skipped,
                } => {
                    new_logs.push(format!(
                        "=== Task Completed: Saved {}, Skipped {} ===",
                        total_downloaded, total_skipped
                    ));
                }
            }
        }
        for log in new_logs {
            state.add_log(log);
        }
    }

    // 2. PostgreSQL Connection Results
    let mut db_results = Vec::new();
    if let Some(ref mut rx) = state.db_rx {
        while let Ok(result) = rx.try_recv() {
            db_results.push(result);
        }
    }
    for result in db_results {
        state.is_reconnecting_db.store(false, Ordering::Relaxed);
        match result {
            Ok(pool) => {
                state.pool = Some(pool);
                state.form_viewer.filings_loaded = false;
                state.form_viewer.template_loaded = false;
                state.analytics.data_loaded = false;
                state.analytics.is_loading = false;
                state.geo_state.markers.clear();
                state.geo_state.is_loading = false;
                state.add_log("[SUCCESS] PostgreSQL connected successfully.".to_string());
            }
            Err(e) => {
                state.add_log(format!("[ERROR] PostgreSQL connection failed: {}", e));
                state.security_feedback_msg = Some((e, true));
            }
        }
    }

    // 3. Analytics Metrics
    let mut analytics_results = Vec::new();
    if let Some(ref mut rx) = state.analytics_rx {
        while let Ok(res) = rx.try_recv() {
            analytics_results.push(res);
        }
    }
    for res in analytics_results {
        state.analytics.is_loading = false;
        match res {
            Ok(metrics) => {
                let mut unique: Vec<(String, String)> = Vec::new();
                for m in &metrics {
                    if !unique.iter().any(|(c, _)| c == &m.corporate_id_number) {
                        unique.push((m.corporate_id_number.clone(), m.company_name.clone()));
                    }
                }
                state.analytics.unique_companies = unique;
                state.analytics.metrics = metrics;
                state.analytics.data_loaded = true;

                let is_valid = state
                    .analytics
                    .unique_companies
                    .iter()
                    .any(|(c, _)| c == &state.analytics.selected_cin);

                if !is_valid {
                    if let Some((first_cin, _)) = state.analytics.unique_companies.first() {
                        state.analytics.selected_cin = first_cin.clone();
                    } else {
                        state.analytics.selected_cin.clear();
                    }
                }
            }
            Err(e) => {
                state.add_log(format!("[ERROR] Analytics load failed: {}", e));
            }
        }
    }

    // 4. Form Viewer Filings, Templates & Facts
    let mut form_events = Vec::new();
    if let Some(ref mut rx) = state.form_rx {
        while let Ok(evt) = rx.try_recv() {
            form_events.push(evt);
        }
    }
    for evt in form_events {
        match evt {
            FormViewerEvent::FilingsLoaded(filings) => {
                let mut unique: Vec<(String, String)> = Vec::new();
                for f in &filings {
                    if !unique.iter().any(|(cin, _)| cin == &f.corporate_id_number) {
                        unique.push((f.corporate_id_number.clone(), f.company_name.clone()));
                    }
                }
                state.form_viewer.unique_companies = unique;

                let already_valid = state
                    .form_viewer
                    .selected_entity_id
                    .is_some_and(|id| filings.iter().any(|f| f.entity_id == id));

                if !already_valid && let Some(first) = filings.first() {
                    state.form_viewer.selected_cin = first.corporate_id_number.clone();
                    state.form_viewer.selected_entity_id = Some(first.entity_id);
                }
                state.form_viewer.available_filings = filings;
            }
            FormViewerEvent::TemplateLoaded(template) => {
                state.form_viewer.form_template = template;
                state.form_viewer.template_loaded = true;
            }
            FormViewerEvent::FactsLoaded { entity_id, facts } => {
                state.form_viewer.cached_entity_id = Some(entity_id);
                state.form_viewer.current_facts = facts;
                state.form_viewer.is_loading_facts = false;
            }
            FormViewerEvent::Error(err) => {
                state.form_viewer.is_loading_facts = false;
                state.add_log(format!("[ERROR] Form Viewer: {}", err));
            }
        }
    }

    // 5. Container Status
    if let Some(ref mut rx) = state.container_rx {
        while let Ok(status) = rx.try_recv() {
            state.container_status = status;
        }
    }

    // 6. AI Copilot Chat & Telemetry
    if let Some(ref mut rx) = state.ai.chat_rx {
        while let Ok(msg) = rx.try_recv() {
            state.ai.chat_history.push(msg);
        }
    }
    if let Some(ref mut rx) = state.ai.telemetry_rx {
        while let Ok(telem) = rx.try_recv() {
            state.ai.last_telemetry = Some(telem);
        }
    }

    // 7. Geo Entity Markers
    let mut geo_results = Vec::new();
    if let Some(ref mut rx) = state.geo_rx {
        while let Ok(res) = rx.try_recv() {
            geo_results.push(res);
        }
    }
    for res in geo_results {
        state.geo_state.is_loading = false;
        match res {
            Ok(markers) => {
                state.geo_state.markers = markers;
            }
            Err(e) => {
                state.add_log(format!("[ERROR] Geo markers load failed: {}", e));
            }
        }
    }

    // 8. Export Feedback Banners
    if let Some(ref mut rx) = state.export_rx {
        while let Ok(feedback) = rx.try_recv() {
            state.export_state.feedback_message = Some(feedback);
        }
    }
}
