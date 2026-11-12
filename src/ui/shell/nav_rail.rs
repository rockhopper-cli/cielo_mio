// src/ui/shell/nav_rail.rs

use crate::ui::state::{ActiveTab, AppState};
use eframe::egui::{self, Color32, Frame, Margin, Panel, RichText, Ui, Vec2};
use std::sync::atomic::Ordering;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let rail_width = if state.nav_collapsed { 64.0 } else { 208.0 };

    Panel::left("left_nav_rail")
        .exact_size(rail_width)
        .resizable(false)
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(6, 8))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Header: Toggle Button & Scaled Logo
                        ui.horizontal(|ui| {
                            if !state.nav_collapsed {
                                let logo_size = egui::vec2(88.0, 88.0);
                                ui.add(
                                    egui::Image::new(egui::include_image!(
                                        "../../../assets/logo.png"
                                    ))
                                    .fit_to_exact_size(logo_size),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |ui| {
                                        if ui
                                            .button(
                                                RichText::new("◀")
                                                    .small()
                                                    .color(Color32::from_rgb(160, 170, 190)),
                                            )
                                            .on_hover_text("Collapse Navigation Rail")
                                            .clicked()
                                        {
                                            state.nav_collapsed = true;
                                        }
                                    },
                                );
                            } else {
                                ui.vertical_centered(|ui| {
                                    if ui
                                        .button(
                                            RichText::new("▶")
                                                .small()
                                                .color(Color32::from_rgb(160, 170, 190)),
                                        )
                                        .on_hover_text("Expand Navigation Rail")
                                        .clicked()
                                    {
                                        state.nav_collapsed = false;
                                    }

                                    ui.add_space(6.0);

                                    let logo_size = egui::vec2(32.0, 32.0);
                                    ui.add(
                                        egui::Image::new(egui::include_image!(
                                            "../../../assets/logo.png"
                                        ))
                                        .fit_to_exact_size(logo_size),
                                    );
                                });
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let nav_items = [
                            (ActiveTab::Download, "📥", "Download & Sync"),
                            (ActiveTab::FormViewer, "📄", "BRSR Form Explorer"),
                            (ActiveTab::Analytics, "📊", "ESG Analytics"),
                            (ActiveTab::GeoReports, "🗺", "Geo Reports"),
                            (ActiveTab::Export, "📤", "Data & DB Export"),
                            (ActiveTab::Config, "⚙", "Settings"),
                            (ActiveTab::About, "ℹ", "About"),
                        ];

                        for (tab, icon, label) in nav_items {
                            let is_active = state.active_tab == tab;
                            let bg_color = if is_active {
                                Color32::from_rgb(38, 48, 62)
                            } else {
                                Color32::TRANSPARENT
                            };
                            let text_color = if is_active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(180, 185, 195)
                            };

                            Frame::NONE
                                .fill(bg_color)
                                .corner_radius(6)
                                .inner_margin(if state.nav_collapsed {
                                    Margin::symmetric(4, 7)
                                } else {
                                    Margin::symmetric(8, 7)
                                })
                                .show(ui, |ui| {
                                    let btn_text = if state.nav_collapsed {
                                        RichText::new(icon).size(16.0).color(text_color)
                                    } else {
                                        RichText::new(format!("{}  {}", icon, label))
                                            .size(13.0)
                                            .strong()
                                            .color(text_color)
                                    };

                                    let resp = ui
                                        .add(
                                            egui::Button::new(btn_text)
                                                .frame(false)
                                                .min_size(Vec2::new(ui.available_width(), 24.0)),
                                        )
                                        .on_hover_text(label);

                                    if resp.clicked() {
                                        state.active_tab = tab;
                                    }
                                });

                            ui.add_space(4.0);
                        }

                        // Status Footer
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(8.0);

                            if state.nav_collapsed {
                                if state.pool.is_some() {
                                    ui.label(
                                        RichText::new("●")
                                            .size(16.0)
                                            .color(Color32::from_rgb(70, 210, 110)),
                                    )
                                    .on_hover_text("Database: Connected");
                                } else {
                                    ui.label(
                                        RichText::new("○")
                                            .size(16.0)
                                            .color(Color32::from_rgb(230, 80, 80)),
                                    )
                                    .on_hover_text("Database: Disconnected");
                                }
                            } else if state.pool.is_some() {
                                ui.label(
                                    RichText::new("● DB Connected")
                                        .small()
                                        .color(Color32::from_rgb(70, 210, 110)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("○ DB Disconnected")
                                        .small()
                                        .color(Color32::from_rgb(230, 80, 80)),
                                );

                                let is_busy = state.is_reconnecting_db.load(Ordering::Relaxed);
                                if is_busy {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label(RichText::new("Connecting...").small());
                                    });
                                } else if ui
                                    .button(
                                        RichText::new("⚡ Start / Connect")
                                            .small()
                                            .color(Color32::from_rgb(100, 180, 240)),
                                    )
                                    .clicked()
                                {
                                    crate::ui::state::trigger_db_start_and_connect(state);
                                }
                            }

                            ui.separator();
                        });
                    });
                });
        });
}
