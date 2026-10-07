// src/app/mod.rs

pub mod ai_drawer;
pub mod logs_panel;
pub mod state;
pub mod tab_analytics;
pub mod tab_config;
pub mod tab_download;
pub mod tab_form_viewer;

use eframe::egui::{self, Color32, Margin, Panel, RichText, Vec2};
use state::{ActiveTab, AppState, FormViewerEvent};
use std::sync::atomic::Ordering;

pub struct BrsrApp {
    pub state: AppState,
}

impl BrsrApp {
    pub fn new(cc: &eframe::CreationContext<'_>, state: AppState) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(15, 17, 21);
        visuals.panel_fill = Color32::from_rgb(20, 23, 28);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 28, 35);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 33, 41);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 49, 61);
        visuals.widgets.active.bg_fill = Color32::from_rgb(52, 62, 78);
        cc.egui_ctx.set_visuals(visuals);

        Self { state }
    }
}

impl eframe::App for BrsrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 1. Drain incoming background activity logs
        let mut new_logs = Vec::new();
        if let Some(ref mut rx) = self.state.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::downloader::types::DownloadEvent::Log(msg) => {
                        if msg.contains("Ingestion Complete") {
                            self.state.form_viewer.filings_loaded = false;
                            self.state.analytics.data_loaded = false;
                        }
                        new_logs.push(format!("[LOG] {}", msg));
                    }
                    crate::downloader::types::DownloadEvent::FileDownloaded { file_name, path } => {
                        new_logs.push(format!("[SAVED] {} -> {}", file_name, path.display()));
                    }
                    crate::downloader::types::DownloadEvent::FileSkipped { file_name, reason } => {
                        new_logs.push(format!("[SKIPPED] {}: {}", file_name, reason));
                    }
                    crate::downloader::types::DownloadEvent::FileFailed { file_name, error } => {
                        new_logs.push(format!("[FAILED] {}: {}", file_name, error));
                    }
                    crate::downloader::types::DownloadEvent::Done {
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
        }
        for log in new_logs {
            self.state.add_log(log);
        }

        // 2. Drain incoming PostgreSQL Connection Task Results
        let mut db_results = Vec::new();
        if let Some(ref mut rx) = self.state.db_rx {
            while let Ok(result) = rx.try_recv() {
                db_results.push(result);
            }
        }

        for result in db_results {
            self.state
                .is_reconnecting_db
                .store(false, Ordering::Relaxed);
            match result {
                Ok(pool) => {
                    self.state.pool = Some(pool);
                    self.state.form_viewer.filings_loaded = false;
                    self.state.form_viewer.template_loaded = false; // Reload template if connected from offline
                    self.state.analytics.data_loaded = false;
                    self.state
                        .add_log("[SUCCESS] PostgreSQL connected successfully.".to_string());
                }
                Err(e) => {
                    self.state
                        .add_log(format!("[ERROR] PostgreSQL connection failed: {}", e));
                    self.state.security_feedback_msg = Some((e, true));
                }
            }
        }

        // 3. Drain incoming Analytics Metrics (Pre-computes unique entities once)
        if let Some(ref mut rx) = self.state.analytics_rx {
            while let Ok(metrics) = rx.try_recv() {
                let mut unique: Vec<(String, String)> = Vec::new();
                for m in &metrics {
                    if !unique.iter().any(|(c, _)| c == &m.corporate_id_number) {
                        unique.push((m.corporate_id_number.clone(), m.company_name.clone()));
                    }
                }
                self.state.analytics.unique_companies = unique;
                self.state.analytics.metrics = metrics;
                self.state.analytics.is_loading = false;
                self.state.analytics.data_loaded = true;
            }
        }

        // 4. Drain incoming Form Viewer events
        let mut form_events = Vec::new();
        if let Some(ref mut form_rx) = self.state.form_rx {
            while let Ok(evt) = form_rx.try_recv() {
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
                    self.state.form_viewer.unique_companies = unique;

                    // Retain entity if already selected (e.g. through Analytics navigation jump)
                    let already_valid = self
                        .state
                        .form_viewer
                        .selected_entity_id
                        .map_or(false, |id| filings.iter().any(|f| f.entity_id == id));

                    if !already_valid {
                        if let Some(first) = filings.first() {
                            self.state.form_viewer.selected_cin = first.corporate_id_number.clone();
                            self.state.form_viewer.selected_entity_id = Some(first.entity_id);
                        }
                    }
                    self.state.form_viewer.available_filings = filings;
                }
                FormViewerEvent::TemplateLoaded(template) => {
                    self.state.form_viewer.form_template = template;
                    self.state.form_viewer.template_loaded = true;
                }
                FormViewerEvent::FactsLoaded { entity_id, facts } => {
                    self.state.form_viewer.cached_entity_id = Some(entity_id);
                    self.state.form_viewer.current_facts = facts;
                    self.state.form_viewer.is_loading_facts = false;
                }
                FormViewerEvent::Error(err) => {
                    self.state.form_viewer.is_loading_facts = false;
                    self.state.add_log(format!("[ERROR] Form Viewer: {}", err));
                }
            }
        }

        // 5. Drain incoming Podman Container Status
        if let Some(ref mut rx) = self.state.container_rx {
            while let Ok(status) = rx.try_recv() {
                self.state.container_status = status;
            }
        }

        // 6. Drain incoming Embedded AI Copilot Responses
        // Drain incoming Embedded AI Copilot Responses
        if let Some(ref mut rx) = self.state.ai.chat_rx {
            while let Ok(msg) = rx.try_recv() {
                self.state.ai.chat_history.push(msg);
            }
        }

        // Drain incoming Copilot Telemetry Diagnostics
        if let Some(ref mut rx) = self.state.ai.telemetry_rx {
            while let Ok(telem) = rx.try_recv() {
                self.state.ai.last_telemetry = Some(telem);
            }
        }

        // 7. Left Navigation Rail (Collapsible with dynamic logo scaling)
        let rail_width = if self.state.nav_collapsed {
            56.0
        } else {
            195.0
        };

        Panel::left("left_nav_rail")
            .exact_size(rail_width)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    // Header: Toggle Button & Scaled Logo
                    ui.horizontal(|ui| {
                        if !self.state.nav_collapsed {
                            let logo_size = egui::vec2(100.0, 100.0);
                            ui.add(
                                egui::Image::new(egui::include_image!("../../assets/logo.png"))
                                    .fit_to_exact_size(logo_size),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui
                                    .button(
                                        RichText::new("◀")
                                            .small()
                                            .color(Color32::from_rgb(160, 170, 190)),
                                    )
                                    .on_hover_text("Collapse Navigation Rail")
                                    .clicked()
                                {
                                    self.state.nav_collapsed = true;
                                }
                            });
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
                                    self.state.nav_collapsed = false;
                                }

                                ui.add_space(6.0);

                                // Scaled down logo in collapsed mode
                                let logo_size = egui::vec2(34.0, 34.0);
                                ui.add(
                                    egui::Image::new(egui::include_image!("../../assets/logo.png"))
                                        .fit_to_exact_size(logo_size),
                                );
                            });
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Nav Items (Expanded: Icon + Title | Collapsed: Centered Icon with Tooltip)
                    let nav_items = [
                        (ActiveTab::Download, "📥", "Download & Sync"),
                        (ActiveTab::FormViewer, "📄", "BRSR Form Explorer"),
                        (ActiveTab::Analytics, "📊", "ESG Analytics"),
                        (ActiveTab::Config, "⚙", "Settings"),
                    ];

                    for (tab, icon, label) in nav_items {
                        let is_active = self.state.active_tab == tab;

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

                        egui::Frame::NONE
                            .fill(bg_color)
                            .corner_radius(6)
                            .inner_margin(if self.state.nav_collapsed {
                                Margin::symmetric(6, 8)
                            } else {
                                Margin::symmetric(10, 8)
                            })
                            .show(ui, |ui| {
                                let btn_text = if self.state.nav_collapsed {
                                    RichText::new(icon).size(16.0).color(text_color)
                                } else {
                                    RichText::new(format!("{}  {}", icon, label))
                                        .size(13.5)
                                        .strong()
                                        .color(text_color)
                                };

                                let resp = ui
                                    .add(
                                        egui::Button::new(btn_text)
                                            .frame(false)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    )
                                    .on_hover_text(label);

                                if resp.clicked() {
                                    self.state.active_tab = tab;
                                }
                            });

                        ui.add_space(4.0);
                    }

                    // Status footer
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(10.0);

                        if self.state.nav_collapsed {
                            // Collapsed status: compact dot with tooltip
                            if self.state.pool.is_some() {
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
                                .on_hover_text("Database: Disconnected (Click to reconnect)");
                            }
                        } else {
                            // Expanded status: full text labels & reconnect button
                            if self.state.pool.is_some() {
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

                                let is_busy = self.state.is_reconnecting_db.load(Ordering::Relaxed);
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
                                    let c_name = self.state.config.db_container_name.clone();
                                    if !crate::container::is_container_running(&c_name) {
                                        let _ = crate::container::start_container(&c_name);
                                    }
                                    state::trigger_db_reconnect(&mut self.state);
                                }
                            }
                        }

                        ui.separator();
                    });
                });
            });

        // 8. Global Reusable Collapsible Logs Panel
        logs_panel::render(ui, &mut self.state);

        // 9. Central Content Panel
        egui::CentralPanel::default().show(ui, |ui| match self.state.active_tab {
            ActiveTab::Download => tab_download::render(ui, &mut self.state),
            ActiveTab::FormViewer => tab_form_viewer::render(ui, &mut self.state),
            ActiveTab::Analytics => tab_analytics::render(ui, &mut self.state),
            ActiveTab::Config => tab_config::render(ui, &mut self.state),
        });

        // 10. Request repaint with ~25 FPS pacing while background tasks are active
        if self.state.is_downloading.load(Ordering::Relaxed)
            || self.state.is_ingesting.load(Ordering::Relaxed)
            || self.state.ai.is_thinking.load(Ordering::Relaxed)
            || self.state.is_reconnecting_db.load(Ordering::Relaxed)
            || self.state.form_viewer.is_loading_facts
            || self.state.analytics.is_loading
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
    }
}

impl Drop for BrsrApp {
    fn drop(&mut self) {
        let _ = self.state.config.save();

        if self.state.config.manage_ollama_container {
            let container_name = &self.state.config.ollama_container_name;
            let _ = crate::container::stop_container(container_name);
        }

        if self.state.config.manage_db_container {
            let container_name = &self.state.config.db_container_name;
            let _ = crate::container::stop_container(container_name);
        }
    }
}
