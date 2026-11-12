// src/ui/shell/logs_panel.rs

use crate::ui::state::AppState;
use eframe::egui::{self, Color32, Frame, Margin, Panel, RichText, ScrollArea, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    Panel::bottom("global_logs_panel").show(ui, |ui| {
        Frame::NONE
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                if !state.show_logs {
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new(format!(
                                    "▶ Console Logs ({})",
                                    state.log_messages.len()
                                ))
                                .small()
                                .strong(),
                            )
                            .clicked()
                        {
                            state.show_logs = true;
                        }

                        ui.separator();

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

                            // UTF-8 safe truncation by char count
                            let truncated: String = if last.chars().count() > 110 {
                                format!("{}...", last.chars().take(110).collect::<String>())
                            } else {
                                last.clone()
                            };

                            ui.add(
                                egui::Label::new(RichText::new(truncated).small().color(color))
                                    .truncate(),
                            );
                        } else {
                            ui.label(RichText::new("System Ready").small().color(Color32::GRAY));
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new(format!(
                                    "▼ Console Logs ({})",
                                    state.log_messages.len()
                                ))
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
                                    let color = if line.contains("[LOG]") {
                                        Color32::from_rgb(180, 190, 205)
                                    } else if line.contains("[SAVED]") || line.contains("[SUCCESS]")
                                    {
                                        Color32::from_rgb(90, 220, 130)
                                    } else if line.contains("[FAILED]") || line.contains("[ERROR]")
                                    {
                                        Color32::from_rgb(240, 90, 90)
                                    } else if line.contains("[SKIPPED]") || line.contains("[WARN]")
                                    {
                                        Color32::from_rgb(220, 180, 80)
                                    } else {
                                        Color32::LIGHT_GRAY
                                    };

                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(line).small().monospace().color(color),
                                        )
                                        .wrap(),
                                    );
                                }
                            }
                        });
                }
            });
    });
}
