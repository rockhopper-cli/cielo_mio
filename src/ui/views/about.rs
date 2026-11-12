// src/ui/views/about.rs

use crate::ui::state::AppState;
use eframe::egui::{self, Color32, Frame, Margin, RichText, ScrollArea, Stroke, Ui};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const APP_LICENSE: &str = env!("CARGO_PKG_LICENSE");

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let display_title = APP_NAME.replace('_', " ").to_uppercase();

    ui.heading(format!("ℹ About {}", display_title));
    ui.label(
        RichText::new("SEBI BRSR Statutory Ingestion, Normalization & ESG Analytics Suite")
            .small()
            .color(Color32::from_rgb(140, 150, 170)),
    );
    ui.add_space(8.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(22, 26, 35))
                .stroke(Stroke::new(1.0, Color32::from_rgb(45, 55, 75)))
                .corner_radius(8)
                .inner_margin(Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let logo_size = egui::vec2(64.0, 64.0);
                        ui.add(
                            egui::Image::new(egui::include_image!("../../../assets/logo.png"))
                                .fit_to_exact_size(logo_size),
                        );

                        ui.add_space(12.0);

                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&display_title)
                                        .size(20.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                );

                                Frame::NONE
                                    .fill(Color32::from_rgb(32, 55, 45))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(70, 180, 110)))
                                    .corner_radius(4)
                                    .inner_margin(Margin::symmetric(8, 2))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("v{}", APP_VERSION))
                                                .strong()
                                                .size(12.0)
                                                .color(Color32::from_rgb(90, 220, 130)),
                                        );
                                    });

                                Frame::NONE
                                    .fill(Color32::from_rgb(30, 42, 60))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(70, 130, 210)))
                                    .corner_radius(4)
                                    .inner_margin(Margin::symmetric(8, 2))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("Edition 2024")
                                                .size(12.0)
                                                .color(Color32::from_rgb(120, 190, 255)),
                                        );
                                    });
                            });

                            ui.add_space(3.0);
                            ui.label(
                                RichText::new(APP_DESCRIPTION)
                                    .small()
                                    .color(Color32::from_rgb(180, 190, 205)),
                            );
                            ui.label(
                                RichText::new(format!("Distributed under the {} License", APP_LICENSE))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        });
                    });
                });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("📐 Pipeline & Engine Architecture").strong());
                ui.add_space(6.0);

                egui::Grid::new("about_tech_grid")
                    .striped(true)
                    .min_col_width(140.0)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Core Engine").strong());
                        ui.add(egui::Label::new("Rust 1.85+ (Native Async tokio runtime, zero-copy quick-xml 0.42)").wrap());
                        ui.end_row();

                        ui.label(RichText::new("Storage Backend").strong());
                        ui.add(egui::Label::new("PostgreSQL 18+ via sqlx 0.9 with rust_decimal & UUIDv7").wrap());
                        ui.end_row();

                        ui.label(RichText::new("GUI Framework").strong());
                        ui.add(egui::Label::new("Immediate-mode native egui 0.37.0, eframe & egui_extras TableBuilder").wrap());
                        ui.end_row();

                        ui.label(RichText::new("Exchange Drivers").strong());
                        ui.add(egui::Label::new("Simultaneous Dual Sync: BSE (JSON + CorpXbrlGen) & NSE (session warmup)").wrap());
                        ui.end_row();

                        ui.label(RichText::new("Security").strong());
                        ui.add(egui::Label::new("OS Secure Keyring (FreeDesktop Secret Service / KWallet / WinCred)").wrap());
                        ui.end_row();

                        ui.label(RichText::new("AI Copilot").strong());
                        ui.add(egui::Label::new("Grounded RAG (Local Ollama qwen2.5-coder / Google Gemini v1beta)").wrap());
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("📜 SEBI BRSR Regulatory Alignment").strong());
                ui.add_space(4.0);

                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "• SEBI Master Circular on Business Responsibility and Sustainability Reporting (BRSR)\n\
                             • BRSR Core Key Performance Indicators (Reasonable / Limited Assessment & Assurance)\n\
                             • MoEFCC Green Credit Rules (Principle 6 Leadership Indicator 8)\n\
                             • Value Chain ESG Thresholds (≥ 2% of total purchases/sales up to 75% coverage ceiling)\n\
                             • Canonical Physical Engineering Conversions: Gigajoules (GJ), Kiloliters (KL), Metric Tonnes (t)"
                        )
                        .small()
                        .color(Color32::from_rgb(190, 200, 215)),
                    )
                    .wrap(),
                );
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("⚙ Active Environment Diagnostics").strong());
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label("Database Status:");
                    if state.pool.is_some() {
                        ui.label(RichText::new("● Connected").color(Color32::from_rgb(70, 210, 110)).strong());
                    } else {
                        ui.label(RichText::new("○ Disconnected").color(Color32::from_rgb(230, 80, 80)).strong());
                    }

                    ui.separator();

                    ui.label("Config Path:");
                    ui.monospace(crate::system::config::AppConfig::config_path().display().to_string());
                });

                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label("Target Architecture:");
                    ui.monospace(format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH));

                    ui.separator();

                    ui.label("In-Memory Disclosures Loaded:");
                    ui.monospace(format!("{} records", state.form_viewer.current_facts.len()));
                });
            });
        });
}
