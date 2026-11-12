// src/ui/views/config.rs

use crate::system::container;
use crate::system::security::{KEY_DB_PASSWORD, KEY_GEMINI_API, SecretManager};
use crate::ui::state::{self, AppState};
use eframe::egui::{self, Color32, ComboBox, DragValue, Frame, Id, Margin, RichText, TextEdit, Ui};
use std::sync::atomic::Ordering;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("⚙ System Configuration");
    ui.label(
        RichText::new("✓ Settings are automatically persisted upon completing edits")
            .small()
            .color(Color32::from_rgb(100, 200, 130)),
    );
    ui.add_space(8.0);

    let mut clear_feedback = false;
    if let Some((ref msg, is_err)) = state.security_feedback_msg {
        let color = if is_err {
            Color32::from_rgb(240, 90, 90)
        } else {
            Color32::from_rgb(90, 220, 130)
        };
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new(RichText::new(msg).color(color).strong()).wrap());
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

    let last_check = ui.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(Id::unique("last_db_pod_check_time"), || {
            std::time::Instant::now() - std::time::Duration::from_secs(10)
        })
    });
    if last_check.elapsed() > std::time::Duration::from_millis(3500) {
        state::trigger_db_container_check(state);
        ui.data_mut(|d| {
            d.insert_temp(
                Id::unique("last_db_pod_check_time"),
                std::time::Instant::now(),
            )
        });
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(2, 2))
                .show(ui, |ui| {
                    // SECTION 1: PostgreSQL
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("🐘 PostgreSQL Database & Container Control").strong(),
                        );
                        ui.add_space(6.0);

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

                            if state.pool.is_some()
                                && ui
                                    .button(
                                        RichText::new("🏷 Reclassify Industries")
                                            .small()
                                            .color(Color32::from_rgb(130, 210, 160)),
                                    )
                                    .on_hover_text(
                                        "Re-run multi-key & NIC industry classification on unclassified rows",
                                    )
                                    .clicked()
                                && let Some(pool) = state.pool.clone()
                            {
                                let tx = state.event_tx.clone();
                                tokio::spawn(async move {
                                    match crate::storage::repository::entities::reclassify_unclassified_entities(&pool).await {
                                        Ok(n) => {
                                            let _ = tx.send(crate::domain::download::DownloadEvent::Log(format!(
                                                "[SUCCESS] Reclassified {} entity records (Ingestion Complete refresh).",
                                                n
                                            ))).await;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(crate::domain::download::DownloadEvent::Log(format!(
                                                "[ERROR] Reclassification failed: {}",
                                                e
                                            ))).await;
                                        }
                                    }
                                });
                            }
                        });

                        ui.add_space(6.0);

                        let db_pod_running = state.db_container_running.load(Ordering::Relaxed);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Pod / Container Name:");
                            let name_resp = ui.add(
                                TextEdit::singleline(&mut state.config.db_container_name)
                                    .desired_width(140.0),
                            );
                            if name_resp.lost_focus() {
                                let _ = state.config.save();
                                state::trigger_db_container_check(state);
                            }

                            if db_pod_running {
                                ui.label(
                                    RichText::new("● Container Running")
                                        .color(Color32::from_rgb(70, 210, 110)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("○ Container Stopped")
                                        .color(Color32::from_rgb(230, 80, 80)),
                                );
                            }

                            ui.separator();

                            if ui
                                .button(
                                    RichText::new("▶ Start & Connect")
                                        .color(Color32::from_rgb(80, 210, 120)),
                                )
                                .clicked()
                            {
                                state::trigger_db_start_and_connect(state);
                            }

                            if ui
                                .button(
                                    RichText::new("⏹ Stop Container")
                                        .color(Color32::from_rgb(240, 90, 90)),
                                )
                                .clicked()
                            {
                                let c_name = state.config.db_container_name.clone();
                                let running_flag = state.db_container_running.clone();
                                let event_tx = state.event_tx.clone();
                                state.pool = None;
                                tokio::spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        container::stop_container(&c_name)
                                    })
                                    .await;
                                    match res {
                                        Ok(Ok(msg)) => {
                                            running_flag.store(false, Ordering::Relaxed);
                                            let _ = event_tx
                                                .send(crate::domain::download::DownloadEvent::Log(
                                                    msg,
                                                ))
                                                .await;
                                        }
                                        Ok(Err(e)) => {
                                            let _ = event_tx
                                                .send(crate::domain::download::DownloadEvent::Log(
                                                    format!("[ERROR] {}", e),
                                                ))
                                                .await;
                                        }
                                        Err(e) => {
                                            let _ = event_tx
                                                .send(crate::domain::download::DownloadEvent::Log(
                                                    format!("[ERROR] {}", e),
                                                ))
                                                .await;
                                        }
                                    }
                                });
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

                        ui.horizontal_wrapped(|ui| {
                            ui.label("DB Host:");
                            let host_resp = ui.add(
                                TextEdit::singleline(&mut state.config.db_host)
                                    .desired_width(160.0),
                            );
                            if host_resp.lost_focus() {
                                let _ = state.config.save();
                            }

                            ui.label("Port:");
                            let port_resp = ui
                                .add(DragValue::new(&mut state.config.db_port).range(1024..=65535));
                            if port_resp.lost_focus() || port_resp.drag_stopped() {
                                let _ = state.config.save();
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("DB User:");
                            let user_resp = ui.add(
                                TextEdit::singleline(&mut state.config.db_user)
                                    .desired_width(160.0),
                            );
                            if user_resp.lost_focus() {
                                let _ = state.config.save();
                            }

                            ui.label("DB Name:");
                            let db_resp = ui.add(
                                TextEdit::singleline(&mut state.config.db_name)
                                    .desired_width(160.0),
                            );
                            if db_resp.lost_focus() {
                                let _ = state.config.save();
                            }
                        });

                        ui.add_space(6.0);

                        let db_persisted = ui.data_mut(|d| {
                            *d.get_temp_mut_or_insert_with(Id::unique("db_pwd_persisted"), || {
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
                                match SecretManager::save_secret(
                                    KEY_DB_PASSWORD,
                                    &state.db_password_input,
                                ) {
                                    Ok(_) => {
                                        state.security_feedback_msg = Some((
                                            "✓ DB password stored in OS keyring.".to_string(),
                                            false,
                                        ));
                                        ui.data_mut(|d| {
                                            d.insert_temp(Id::unique("db_pwd_persisted"), true)
                                        });
                                    }
                                    Err(e) => state.security_feedback_msg = Some((e, true)),
                                }
                            }

                            if db_persisted && ui.button("🗑 Remove").clicked() {
                                let _ = SecretManager::delete_secret(KEY_DB_PASSWORD);
                                state.db_password_input.clear();
                                state.security_feedback_msg =
                                    Some(("DB password removed from keyring.".to_string(), false));
                                ui.data_mut(|d| {
                                    d.insert_temp(Id::unique("db_pwd_persisted"), false)
                                });
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // SECTION 2: Ollama Container
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("🐳 Local Ollama & Container Lifecycle").strong(),
                        );
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
                            let url_resp = ui.add(
                                TextEdit::singleline(&mut state.config.ollama_url)
                                    .desired_width(180.0),
                            );
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
                                    RichText::new("● Container Running")
                                        .color(Color32::from_rgb(70, 210, 110)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("○ Container Stopped")
                                        .color(Color32::from_rgb(230, 80, 80)),
                                );
                            }

                            ui.separator();

                            if state.container_status.service_online {
                                ui.label(
                                    RichText::new("● HTTP Online")
                                        .color(Color32::from_rgb(70, 210, 110)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("○ HTTP Offline")
                                        .color(Color32::from_rgb(230, 80, 80)),
                                );
                            }

                            ui.separator();

                            if ui
                                .button(
                                    RichText::new("▶ Start").color(Color32::from_rgb(80, 210, 120)),
                                )
                                .clicked()
                            {
                                trigger_ollama_action(state, true);
                            }

                            if ui
                                .button(
                                    RichText::new("⏹ Stop").color(Color32::from_rgb(240, 90, 90)),
                                )
                                .clicked()
                            {
                                trigger_ollama_action(state, false);
                            }

                            if ui.button("🔄 Health Check").clicked() {
                                trigger_health_check(state);
                            }
                        });

                        if !state.container_status.available_models.is_empty() {
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Model:");
                                let prev_model = state.config.ollama_model.clone();
                                ComboBox::from_id_salt("ollama_model_combo")
                                    .width(240.0)
                                    .selected_text(&state.config.ollama_model)
                                    .show_ui(ui, |ui| {
                                        for m in &state.container_status.available_models {
                                            ui.selectable_value(
                                                &mut state.config.ollama_model,
                                                m.clone(),
                                                m,
                                            );
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

                    // SECTION 3: Gemini
                    ui.group(|ui| {
                        ui.label(RichText::new("☁ Google Gemini API Configuration").strong());
                        ui.add_space(6.0);

                        let gemini_persisted = ui.data_mut(|d| {
                            *d.get_temp_mut_or_insert_with(
                                Id::unique("gemini_key_persisted"),
                                || SecretManager::is_persisted_in_keyring(KEY_GEMINI_API),
                            )
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
                                match SecretManager::save_secret(
                                    KEY_GEMINI_API,
                                    &state.ai.gemini_key,
                                ) {
                                    Ok(_) => {
                                        state.security_feedback_msg = Some((
                                            "✓ Gemini API Key saved to OS Keyring.".to_string(),
                                            false,
                                        ));
                                        ui.data_mut(|d| {
                                            d.insert_temp(Id::unique("gemini_key_persisted"), true)
                                        });
                                    }
                                    Err(e) => state.security_feedback_msg = Some((e, true)),
                                }
                            }

                            if gemini_persisted && ui.button("🗑 Remove").clicked() {
                                let _ = SecretManager::delete_secret(KEY_GEMINI_API);
                                state.ai.gemini_key.clear();
                                state.security_feedback_msg = Some((
                                    "Gemini API Key removed from OS Keyring.".to_string(),
                                    false,
                                ));
                                ui.data_mut(|d| {
                                    d.insert_temp(Id::unique("gemini_key_persisted"), false)
                                });
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Model Tag:");
                            let gem_resp = ui.add(
                                TextEdit::singleline(&mut state.config.gemini_model)
                                    .desired_width(180.0),
                            );
                            if gem_resp.lost_focus() {
                                state.ai.gemini_model = state.config.gemini_model.clone();
                                let _ = state.config.save();
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // SECTION 4: Pipeline Defaults
                    ui.group(|ui| {
                        ui.label(RichText::new("📥 Pipeline & Ingestion Defaults").strong());
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Target Folder:");
                            ui.monospace(state.config.default_output_dir.display().to_string());
                            if ui.button("Browse...").clicked()
                                && let Some(folder) = rfd::FileDialog::new().pick_folder()
                            {
                                state.config.default_output_dir = folder.clone();
                                state.output_dir = folder;
                                let _ = state.config.save();
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
                                        .range(2..=100)
                                        .suffix(" files"),
                                );
                                if cap_resp.lost_focus() || cap_resp.drag_stopped() {
                                    state.test_cap = state.config.test_cap_default;
                                    let _ = state.config.save();
                                }
                            }
                        });
                    });

                    ui.add_space(16.0);
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

fn trigger_ollama_action(state: &AppState, start: bool) {
    let c_name = state.config.ollama_container_name.clone();
    let url = state.config.ollama_url.clone();
    let tx = state.container_tx.clone();
    let event_tx = state.event_tx.clone();

    tokio::spawn(async move {
        let cname_inner = c_name.clone();
        let op_res = tokio::task::spawn_blocking(move || {
            if start {
                container::start_container(&cname_inner)
            } else {
                container::stop_container(&cname_inner)
            }
        })
        .await;

        if let Ok(Err(e)) = op_res {
            let _ = event_tx
                .send(crate::domain::download::DownloadEvent::Log(format!(
                    "[ERROR] {}",
                    e
                )))
                .await;
        }
        let status = container::check_service_health(&c_name, &url).await;
        let _ = tx.send(status).await;
    });
}
