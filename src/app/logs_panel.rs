// src/app/logs_panel.rs

use crate::app::state::AppState;
use eframe::egui::{self, Color32, Panel, RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    Panel::bottom("global_logs_panel").show(ui, |ui| {
        if !state.show_logs {
            // Collapsed slim toolbar with a single toggle button
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(format!("▶ Console Logs ({})", state.log_messages.len()))
                            .small()
                            .strong(),
                    )
                    .clicked()
                {
                    state.show_logs = true;
                }

                ui.separator();

                // Live preview of the latest log line (now using VecDeque::back)
                if let Some(last) = state.log_messages.back() {
                    let color = if last.contains("[ERROR]") || last.contains("[FAILED]") {
                        Color32::from_rgb(240, 90, 90)
                    } else if last.contains("[SUCCESS]") || last.contains("[SAVED]") {
                        Color32::from_rgb(90, 220, 130)
                    } else if last.contains("[SKIPPED]") || last.contains("[WARN]") {
                        Color32::from_rgb(220, 180, 80)
                    } else {
                        Color32::from_rgb(160, 170, 185)
                    };

                    let truncated = if last.len() > 140 {
                        format!("{}...", &last[..140])
                    } else {
                        last.clone()
                    };

                    ui.label(RichText::new(truncated).small().color(color));
                } else {
                    ui.label(RichText::new("System Ready").small().color(Color32::GRAY));
                }
            });
        } else {
            // Expanded console: same button toggles it closed
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(format!("▼ Console Logs ({})", state.log_messages.len()))
                            .small()
                            .strong(),
                    )
                    .clicked()
                {
                    state.show_logs = false;
                }

                ui.separator();

                ui.label(
                    RichText::new("Live Activity Stream")
                        .small()
                        .color(Color32::GRAY),
                );

                // Only functional utilities on the right (no duplicate close button)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🗑 Clear").clicked() {
                        state.log_messages.clear();
                    }
                });
            });

            ui.add_space(4.0);

            ScrollArea::vertical()
                .max_height(160.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if state.log_messages.is_empty() {
                        ui.label(
                            RichText::new("No activity logged yet.")
                                .small()
                                .color(Color32::GRAY),
                        );
                    } else {
                        for line in &state.log_messages {
                            let color = if line.starts_with("[LOG]") {
                                Color32::from_rgb(180, 190, 205)
                            } else if line.starts_with("[SAVED]") || line.starts_with("[SUCCESS]") {
                                Color32::from_rgb(90, 220, 130)
                            } else if line.starts_with("[FAILED]") || line.starts_with("[ERROR]") {
                                Color32::from_rgb(240, 90, 90)
                            } else if line.starts_with("[SKIPPED]") || line.starts_with("[WARN]") {
                                Color32::from_rgb(220, 180, 80)
                            } else {
                                Color32::LIGHT_GRAY
                            };

                            ui.label(RichText::new(line).small().monospace().color(color));
                        }
                    }
                });
        }
    });
}
