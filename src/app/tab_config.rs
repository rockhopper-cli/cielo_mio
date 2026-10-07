// src/app/tab_config.rs

use crate::app::state::{self, AppState};
use crate::container;
use crate::security::{KEY_DB_PASSWORD, KEY_GEMINI_API, SecretManager};
use eframe::egui::{self, Color32, ComboBox, DragValue, Id, RichText, TextEdit, Ui};
use std::sync::atomic::Ordering;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("⚙ System Configuration");
    ui.label(
        RichText::new("✓ Settings are automatically persisted upon completing edits")
            .small()
            .color(Color32::from_rgb(100, 200, 130)),
    );
    ui.add_space(8.0);

    // Feedback Banner
    let mut clear_feedback = false;
    if let Some((ref msg, is_err)) = state.security_feedback_msg {
        let color = if is_err {
            Color32::from_rgb(240, 90, 90)
        } else {
            Color32::from_rgb(90, 220, 130)
        };
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(msg).color(color).strong());
                if ui.button("✖").clicked() {
                    clear_feedback = true;
                }
            });
        });
        ui.add_space(6.0);
    }
    if clear_feedback {
        state.security_feedback_msg = None;
    }

    // Non-blocking throttled poll: checks container status at most once every 3.5 seconds
    let last_check = ui.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(Id::new("last_db_pod_check_time"), || {
            std::time::Instant::now() - std::time::Duration::from_secs(10)
        })
    });
    if last_check.elapsed() > std::time::Duration::from_millis(3500) {
        state::trigger_db_container_check(state);
        ui.data_mut(|d| {
            d.insert_temp(Id::new("last_db_pod_check_time"), std::time::Instant::now())
        });
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // =====================================================================
        // SECTION 1: PostgreSQL Database Connection & Podman Pod Control
        // =====================================================================
        ui.group(|ui| {
            ui.label(RichText::new("🐘 PostgreSQL Database & Podman Pod Control").strong());
            ui.add_space(6.0);

            // Status Bar & Immediate Reconnect Action
            ui.horizontal_wrapped(|ui| {
                if state.pool.is_some() {
                    ui.label(
                        RichText::new("● Database Connected")
                            .color(Color32::from_rgb(70, 210, 110))
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new("○ Database Disconnected")
                            .color(Color32::from_rgb(230, 80, 80))
                            .strong(),
                    );
                }

                ui.separator();

                let is_busy = state.is_reconnecting_db.load(Ordering::Relaxed);
                if is_busy {
                    ui.spinner();
                    ui.label("Testing connection...");
                } else if ui
                    .button(
                        RichText::new("⚡ Test & Connect")
                            .color(Color32::from_rgb(100, 180, 240))
                            .strong(),
                    )
                    .clicked()
                {
                    state::trigger_db_reconnect(state);
                    state::trigger_db_container_check(state);
                }
            });

            ui.add_space(6.0);

            // Instant read from cached atomic boolean (0 OS process spawns per frame)
            let db_pod_running = state.db_container_running.load(Ordering::Relaxed);
            ui.horizontal_wrapped(|ui| {
                ui.label("Pod / Container Name:");
                let name_resp = ui.add(
                    TextEdit::singleline(&mut state.config.db_container_name).desired_width(140.0),
                );
                if name_resp.lost_focus() {
                    let _ = state.config.save();
                    state::trigger_db_container_check(state);
                }

                if db_pod_running {
                    ui.label(RichText::new("● Pod Running").color(Color32::from_rgb(70, 210, 110)));
                } else {
                    ui.label(RichText::new("○ Pod Stopped").color(Color32::from_rgb(230, 80, 80)));
                }

                ui.separator();

                if ui
                    .button(RichText::new("▶ Start Pod").color(Color32::from_rgb(80, 210, 120)))
                    .clicked()
                {
                    match container::start_container(&state.config.db_container_name) {
                        Ok(msg) => {
                            state.add_log(format!("[SUCCESS] {}", msg));
                            state.db_container_running.store(true, Ordering::Relaxed);
                            state::trigger_db_reconnect(state);
                        }
                        Err(e) => {
                            state.add_log(format!("[ERROR] {}", e));
                            state.security_feedback_msg = Some((e, true));
                            state::trigger_db_container_check(state);
                        }
                    }
                }

                if ui
                    .button(RichText::new("⏹ Stop Pod").color(Color32::from_rgb(240, 90, 90)))
                    .clicked()
                {
                    match container::stop_container(&state.config.db_container_name) {
                        Ok(msg) => {
                            state.add_log(format!("[LOG] {}", msg));
                            state.db_container_running.store(false, Ordering::Relaxed);
                            state.pool = None;
                        }
                        Err(e) => {
                            state.add_log(format!("[ERROR] {}", e));
                            state.security_feedback_msg = Some((e, true));
                            state::trigger_db_container_check(state);
                        }
                    }
                }
            });

            ui.add_space(4.0);

            if ui
                .checkbox(
                    &mut state.config.manage_db_container,
                    "Auto-launch DB container on app start and auto-stop on exit",
                )
                .changed()
            {
                let _ = state.config.save();
            }

            ui.add_space(6.0);

            // Host & Port (Persist on lost focus)
            ui.horizontal_wrapped(|ui| {
                ui.label("DB Host:");
                let host_resp =
                    ui.add(TextEdit::singleline(&mut state.config.db_host).desired_width(160.0));
                if host_resp.lost_focus() {
                    let _ = state.config.save();
                }

                ui.label("Port:");
                let port_resp =
                    ui.add(DragValue::new(&mut state.config.db_port).range(1024..=65535));
                if port_resp.lost_focus() || port_resp.drag_stopped() {
                    let _ = state.config.save();
                }
            });

            // User & Database Name (Persist on lost focus)
            ui.horizontal_wrapped(|ui| {
                ui.label("DB User:");
                let user_resp =
                    ui.add(TextEdit::singleline(&mut state.config.db_user).desired_width(160.0));
                if user_resp.lost_focus() {
                    let _ = state.config.save();
                }

                ui.label("DB Name:");
                let db_resp =
                    ui.add(TextEdit::singleline(&mut state.config.db_name).desired_width(160.0));
                if db_resp.lost_focus() {
                    let _ = state.config.save();
                }
            });

            ui.add_space(6.0);

            // Database Password (OS Keyring)
            let db_persisted = ui.data_mut(|d| {
                *d.get_temp_mut_or_insert_with(Id::new("db_pwd_persisted"), || {
                    SecretManager::is_persisted_in_keyring(KEY_DB_PASSWORD)
                })
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("DB Password:");
                ui.add(
                    TextEdit::singleline(&mut state.db_password_input)
                        .password(true)
                        .desired_width(180.0),
                );

                if db_persisted {
                    ui.label(
                        RichText::new("● Stored in Keyring")
                            .color(Color32::from_rgb(70, 210, 110))
                            .small(),
                    );
                } else {
                    ui.label(
                        RichText::new("○ Not in Keyring")
                            .color(Color32::YELLOW)
                            .small(),
                    );
                }

                if ui.button("💾 Save Password").clicked() {
                    match SecretManager::save_secret(KEY_DB_PASSWORD, &state.db_password_input) {
                        Ok(_) => {
                            state.security_feedback_msg =
                                Some(("✓ DB password stored in OS keyring.".to_string(), false));
                            ui.data_mut(|d| d.insert_temp(Id::new("db_pwd_persisted"), true));
                        }
                        Err(e) => state.security_feedback_msg = Some((e, true)),
                    }
                }

                if db_persisted && ui.button("🗑 Remove").clicked() {
                    let _ = SecretManager::delete_secret(KEY_DB_PASSWORD);
                    state.db_password_input.clear();
                    state.security_feedback_msg =
                        Some(("DB password removed from keyring.".to_string(), false));
                    ui.data_mut(|d| d.insert_temp(Id::new("db_pwd_persisted"), false));
                }
            });
        });

        ui.add_space(8.0);

        // =====================================================================
        // SECTION 2: Local LLM & Podman Container Lifecycle
        // =====================================================================
        ui.group(|ui| {
            ui.label(RichText::new("🐳 Local Ollama & Podman Container Lifecycle").strong());
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Container Name:");
                let ollama_name_resp = ui.add(
                    TextEdit::singleline(&mut state.config.ollama_container_name)
                        .desired_width(160.0),
                );
                if ollama_name_resp.lost_focus() {
                    let _ = state.config.save();
                }

                ui.label("URL:");
                let url_resp =
                    ui.add(TextEdit::singleline(&mut state.config.ollama_url).desired_width(180.0));
                if url_resp.lost_focus() {
                    state.ai.ollama_url = state.config.ollama_url.clone();
                    let _ = state.config.save();
                }
            });

            ui.add_space(4.0);

            if ui
                .checkbox(
                    &mut state.config.manage_ollama_container,
                    "Auto-launch on app start and auto-stop on exit",
                )
                .changed()
            {
                let _ = state.config.save();
            }

            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                if state.container_status.container_running {
                    ui.label(
                        RichText::new("● Podman Running").color(Color32::from_rgb(70, 210, 110)),
                    );
                } else {
                    ui.label(
                        RichText::new("○ Podman Stopped").color(Color32::from_rgb(230, 80, 80)),
                    );
                }

                ui.separator();

                if state.container_status.service_online {
                    ui.label(RichText::new("● HTTP Online").color(Color32::from_rgb(70, 210, 110)));
                } else {
                    ui.label(RichText::new("○ HTTP Offline").color(Color32::from_rgb(230, 80, 80)));
                }

                ui.separator();

                if ui
                    .button(RichText::new("▶ Start").color(Color32::from_rgb(80, 210, 120)))
                    .clicked()
                {
                    let _ = container::start_container(&state.config.ollama_container_name);
                    trigger_health_check(state);
                }

                if ui
                    .button(RichText::new("⏹ Stop").color(Color32::from_rgb(240, 90, 90)))
                    .clicked()
                {
                    let _ = container::stop_container(&state.config.ollama_container_name);
                    trigger_health_check(state);
                }

                if ui.button("🔄 Health Check").clicked() {
                    trigger_health_check(state);
                }
            });

            if !state.container_status.available_models.is_empty() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    let prev_model = state.config.ollama_model.clone();
                    ComboBox::from_id_salt("ollama_model_combo")
                        .width(240.0)
                        .selected_text(&state.config.ollama_model)
                        .show_ui(ui, |ui| {
                            for m in &state.container_status.available_models {
                                ui.selectable_value(&mut state.config.ollama_model, m.clone(), m);
                            }
                        });

                    if state.config.ollama_model != prev_model {
                        state.ai.ollama_model = state.config.ollama_model.clone();
                        let _ = state.config.save();
                    }
                });
            }
        });

        ui.add_space(8.0);

        // =====================================================================
        // SECTION 3: Cloud LLM (Gemini) Key & Model
        // =====================================================================
        ui.group(|ui| {
            ui.label(RichText::new("☁ Google Gemini API Configuration").strong());
            ui.add_space(6.0);

            let gemini_persisted = ui.data_mut(|d| {
                *d.get_temp_mut_or_insert_with(Id::new("gemini_key_persisted"), || {
                    SecretManager::is_persisted_in_keyring(KEY_GEMINI_API)
                })
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("API Key:");
                ui.add(
                    TextEdit::singleline(&mut state.ai.gemini_key)
                        .password(true)
                        .desired_width(220.0),
                );

                if gemini_persisted {
                    ui.label(
                        RichText::new("● In Keyring")
                            .color(Color32::from_rgb(70, 210, 110))
                            .small(),
                    );
                } else {
                    ui.label(RichText::new("○ Unsaved").color(Color32::YELLOW).small());
                }

                if ui.button("💾 Save Key").clicked() {
                    match SecretManager::save_secret(KEY_GEMINI_API, &state.ai.gemini_key) {
                        Ok(_) => {
                            state.security_feedback_msg =
                                Some(("✓ Gemini API Key saved to OS Keyring.".to_string(), false));
                            ui.data_mut(|d| d.insert_temp(Id::new("gemini_key_persisted"), true));
                        }
                        Err(e) => state.security_feedback_msg = Some((e, true)),
                    }
                }

                if gemini_persisted && ui.button("🗑 Remove").clicked() {
                    let _ = SecretManager::delete_secret(KEY_GEMINI_API);
                    state.ai.gemini_key.clear();
                    state.security_feedback_msg =
                        Some(("Gemini API Key removed from OS Keyring.".to_string(), false));
                    ui.data_mut(|d| d.insert_temp(Id::new("gemini_key_persisted"), false));
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Model Tag:");
                let gem_resp = ui
                    .add(TextEdit::singleline(&mut state.config.gemini_model).desired_width(180.0));
                if gem_resp.lost_focus() {
                    state.ai.gemini_model = state.config.gemini_model.clone();
                    let _ = state.config.save();
                }
            });
        });

        ui.add_space(8.0);

        // =====================================================================
        // SECTION 4: Pipeline & Downloader Defaults
        // =====================================================================
        ui.group(|ui| {
            ui.label(RichText::new("📥 Pipeline & Ingestion Defaults").strong());
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Target Folder:");
                ui.monospace(state.config.default_output_dir.display().to_string());
                if ui.button("Browse...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        state.config.default_output_dir = folder.clone();
                        state.output_dir = folder;
                        let _ = state.config.save();
                    }
                }
            });

            ui.add_space(4.0);

            if ui
                .checkbox(
                    &mut state.config.auto_ingest_default,
                    "Auto-ingest unmapped XML into PostgreSQL by default",
                )
                .changed()
            {
                state.auto_ingest = state.config.auto_ingest_default;
                let _ = state.config.save();
            }

            ui.horizontal_wrapped(|ui| {
                if ui
                    .checkbox(
                        &mut state.config.test_mode_default,
                        "Enable test limit cap by default:",
                    )
                    .changed()
                {
                    state.test_mode = state.config.test_mode_default;
                    let _ = state.config.save();
                }
                if state.config.test_mode_default {
                    let cap_resp = ui.add(
                        DragValue::new(&mut state.config.test_cap_default)
                            .range(1..=100)
                            .suffix(" files"),
                    );
                    if cap_resp.lost_focus() || cap_resp.drag_stopped() {
                        state.test_cap = state.config.test_cap_default;
                        let _ = state.config.save();
                    }
                }
            });
        });
    });
}

fn trigger_health_check(state: &AppState) {
    let c_name = state.config.ollama_container_name.clone();
    let url = state.config.ollama_url.clone();
    let tx = state.container_tx.clone();

    tokio::spawn(async move {
        let status = container::check_service_health(&c_name, &url).await;
        let _ = tx.send(status).await;
    });
}
