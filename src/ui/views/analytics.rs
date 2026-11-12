// src/ui/views/analytics.rs

pub mod benchmarking;
pub mod charts;
pub mod controls;
pub mod data;
pub mod pillars;

pub use controls::get_active_metric_label;

use crate::ui::shell::ai_drawer::render_ai_copilot_drawer;
use crate::ui::state::AppState;
use eframe::egui::{Color32, RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("📊 BRSR ESG Sustainability Tracker");
    ui.add_space(4.0);

    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    RichText::new("⚠ PostgreSQL Disconnected")
                        .size(18.0)
                        .color(Color32::from_rgb(230, 90, 90)),
                );
                ui.label("Connect your database container to view ESG analytics.");
                ui.add_space(8.0);
                if ui
                    .button(
                        RichText::new("⚡ Attempt Reconnect")
                            .color(Color32::from_rgb(100, 180, 240)),
                    )
                    .clicked()
                {
                    crate::ui::state::trigger_db_reconnect(state);
                }
            });
            return;
        }
    };

    if !state.analytics.data_loaded && !state.analytics.is_loading {
        state.analytics.is_loading = true;
        let pool_cloned = pool.clone();
        let tx = state.analytics_tx.clone();

        tokio::spawn(async move {
            let res = data::fetch_esg_metrics_from_db(&pool_cloned)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(res).await;
        });
    }

    if state.analytics.selected_cin.is_empty()
        && let Some(first) = state.analytics.metrics.first()
    {
        state.analytics.selected_cin = first.corporate_id_number.clone();
    }

    let copilot_hint = format!(
        "Analytics Dashboard | CIN: {} | Pillar: {:?} | Metric: {}",
        if state.analytics.selected_cin.is_empty() {
            "All"
        } else {
            &state.analytics.selected_cin
        },
        state.analytics.selected_pillar,
        get_active_metric_label(state)
    );
    render_ai_copilot_drawer(ui, state, &copilot_hint);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            controls::render_brsr_pillar_cards(ui, state);

            ui.add_space(10.0);

            controls::render_search_and_metric_capsules(ui, state);

            ui.add_space(12.0);

            pillars::render_active_metric_view(ui, state);

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);

            benchmarking::render_benchmarking_table(ui, state);

            ui.add_space(24.0);
        });
}
