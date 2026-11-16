// src/ui/views/export.rs

use crate::ui::state::AppState;
use eframe::egui::{self, Color32, ComboBox, Frame, Margin, RichText, Ui};
use sqlx::Row;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("📤 Data Export & Database Backups");
    ui.add(
        egui::Label::new(
            RichText::new("Export filtered ESG disclosure tables to CSV (UTF-8 / Excel-compatible) or generate standalone PostgreSQL dumps.")
                .small()
                .color(Color32::from_rgb(160, 170, 185)),
        )
        .wrap(),
    );
    ui.add_space(8.0);

    // Ensure entity & sector filter lists load even if user hasn't opened Analytics tab yet
    if let Some(pool) = state.pool.clone()
        && !state.analytics.data_loaded
        && !state.analytics.is_loading
    {
        state.analytics.is_loading = true;
        let tx = state.analytics_tx.clone();
        tokio::spawn(async move {
            let res = crate::storage::repository::analytics::fetch_esg_metrics(&pool)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(res).await;
        });
    }

    let mut clear_feedback = false;
    if let Some((ref msg, is_err)) = state.export_state.feedback_message {
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
        state.export_state.feedback_message = None;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(2, 2))
                .show(ui, |ui| {
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("📊 1. Export Disclosures (CSV Format)").strong(),
                        );
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Encoding:");
                            ui.radio_value(
                                &mut state.export_state.export_format_xlsx,
                                false,
                                "Standard UTF-8 CSV",
                            );
                            ui.radio_value(
                                &mut state.export_state.export_format_xlsx,
                                true,
                                "Excel-Compatible CSV (UTF-8 BOM)",
                            );

                            ui.separator();

                            ui.label("Entity:");
                            let current_ent = if state.export_state.export_scope_entity.is_empty() {
                                "All Entities".to_string()
                            } else {
                                state.export_state.export_scope_entity.clone()
                            };

                            ComboBox::from_id_salt("export_entity_selector")
                                .width(220.0)
                                .selected_text(&current_ent)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.export_state.export_scope_entity,
                                        "".to_string(),
                                        "All Entities",
                                    );
                                    for (cin, name) in &state.analytics.unique_companies {
                                        ui.selectable_value(
                                            &mut state.export_state.export_scope_entity,
                                            cin.clone(),
                                            format!("{} ({})", name, cin),
                                        );
                                    }
                                });

                            ui.label("Sector:");
                            let current_sec = if state.export_state.export_scope_sector.is_empty() {
                                "All Sectors".to_string()
                            } else {
                                state.export_state.export_scope_sector.clone()
                            };

                            ComboBox::from_id_salt("export_sector_selector")
                                .width(180.0)
                                .selected_text(&current_sec)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.export_state.export_scope_sector,
                                        "".to_string(),
                                        "All Sectors",
                                    );
                                    let mut sectors: Vec<String> = state
                                        .analytics
                                        .metrics
                                        .iter()
                                        .map(|m| m.industry_sector.clone())
                                        .collect();
                                    sectors.sort();
                                    sectors.dedup();
                                    for s in sectors {
                                        ui.selectable_value(
                                            &mut state.export_state.export_scope_sector,
                                            s.clone(),
                                            &s,
                                        );
                                    }
                                });

                            ui.label("Grouping:");
                            let current_grp =
                                if state.export_state.export_scope_grouping.is_empty() {
                                    "All Groups".to_string()
                                } else {
                                    state.export_state.export_scope_grouping.clone()
                                };

                            ComboBox::from_id_salt("export_group_selector")
                                .width(180.0)
                                .selected_text(&current_grp)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.export_state.export_scope_grouping,
                                        "".to_string(),
                                        "All Groups",
                                    );

                                    let dynamic_groups: BTreeSet<String> = state
                                        .form_viewer
                                        .form_template
                                        .iter()
                                        .filter_map(|it| it.friendly_grouping.as_deref())
                                        .map(str::trim)
                                        .filter(|s| !s.is_empty())
                                        .map(str::to_string)
                                        .collect();

                                    if dynamic_groups.is_empty() {
                                        let fallback_groups = [
                                            "entity_list",
                                            "details_ghg_emission",
                                            "ghg_scope3_emissions",
                                            "details_energy_consumption",
                                            "details_water_withdrawal",
                                            "details_water_discharge",
                                            "details_waste_management",
                                            "women_representation_management",
                                            "turnover_rate",
                                            "details_safety_incident",
                                            "details_posh",
                                            "greencredits",
                                        ];
                                        for g in fallback_groups {
                                            ui.selectable_value(
                                                &mut state.export_state.export_scope_grouping,
                                                g.to_string(),
                                                g,
                                            );
                                        }
                                    } else {
                                        for g in dynamic_groups {
                                            ui.selectable_value(
                                                &mut state.export_state.export_scope_grouping,
                                                g.clone(),
                                                &g,
                                            );
                                        }
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        let is_busy = state.export_state.is_exporting.load(Ordering::Relaxed);
                        if !is_busy {
                            let has_db = state.pool.is_some();
                            if ui
                                .add_enabled(
                                    has_db,
                                    egui::Button::new(
                                        RichText::new("💾 Save CSV Export ➔")
                                            .color(Color32::from_rgb(100, 180, 240))
                                            .strong(),
                                    ),
                                )
                                .clicked()
                                && let Some(target_path) = rfd::FileDialog::new()
                                    .set_file_name(format!(
                                        "brsr_export_{}.csv",
                                        chrono::Local::now().format("%Y%m%d_%H%M")
                                    ))
                                    .add_filter("CSV File", &["csv"])
                                    .save_file()
                            {
                                trigger_data_export(state, target_path);
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Streaming facts and writing CSV to disk...");
                            });
                        }
                    });

                    ui.add_space(10.0);

                    ui.group(|ui| {
                        ui.label(
                            RichText::new("🐘 2. Standalone PostgreSQL Database Backup").strong(),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "Target Database: '{}' on {}:{} (Container: '{}')",
                                    state.config.db_name,
                                    state.config.db_host,
                                    state.config.db_port,
                                    state.config.db_container_name
                                ))
                                .small()
                                .color(Color32::GRAY),
                            )
                            .wrap(),
                        );

                        ui.add_space(6.0);

                        let is_dumping = state.export_state.is_dumping_db.load(Ordering::Relaxed);
                        if !is_dumping {
                            if ui
                                .button(
                                    RichText::new("📦 Create Full Backup (.dump) ➔")
                                        .color(Color32::from_rgb(90, 210, 140))
                                        .strong(),
                                )
                                .clicked()
                                && let Some(target_path) = rfd::FileDialog::new()
                                    .set_file_name(format!(
                                        "backup_{}_{}.dump",
                                        state.config.db_name,
                                        chrono::Local::now().format("%Y%m%d_%H%M%S")
                                    ))
                                    .add_filter("PostgreSQL Custom Dump", &["dump", "sql", "tar"])
                                    .save_file()
                            {
                                trigger_db_backup(state, target_path);
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(
                                    "Creating PostgreSQL physical backup via pg_dump in background...",
                                );
                            });
                        }
                    });
                });
        });
}

fn csv_escape(s: &str) -> String {
    let cleaned = s.replace(['\r', '\n'], " ");
    format!("\"{}\"", cleaned.replace('"', "\"\""))
}

fn trigger_data_export(state: &mut AppState, dest_path: PathBuf) {
    let pool = match state.pool.clone() {
        Some(p) => p,
        None => return,
    };

    let flag = state.export_state.is_exporting.clone();
    flag.store(true, Ordering::Relaxed);

    let cin_filter = state.export_state.export_scope_entity.clone();
    let sec_filter = state.export_state.export_scope_sector.clone();
    let grp_filter = state.export_state.export_scope_grouping.clone();
    let write_bom = state.export_state.export_format_xlsx;
    let export_tx = state.export_tx.clone();

    tokio::spawn(async move {
        let rows = crate::storage::repository::analytics::fetch_export_facts(
            &pool,
            &cin_filter,
            &sec_filter,
            &grp_filter,
        )
        .await;

        let feedback = match rows {
            Ok(records) => {
                let count = records.len();
                let write_res: std::io::Result<()> = (|| {
                    let file = File::create(&dest_path)?;
                    let mut writer = BufWriter::new(file);
                    if write_bom {
                        writer.write_all("\u{FEFF}".as_bytes())?;
                    }
                    writer.write_all(
                        b"Company,CIN,Sector,Section,Principle,Indicator_Type,Grouping,Metric,Context,Raw_Value,Canonical_Value,Unit,Data_Type\n",
                    )?;
                    for r in records {
                        let comp: String = r.get("company_name");
                        let cin: String = r.get("corporate_id_number");
                        let sec: String = r.get("industry_sector");
                        let s: String = r.get("section");
                        let p: i16 = r.get("principle_number");
                        let ind_type: String = r.get("indicator_type");
                        let grp: String = r.get("friendly_grouping");
                        let tag: String = r.get("standard_indicator_capmkt");
                        let ctx: String = r.get("context_ref");
                        let raw: Option<String> = r.get("raw_value");
                        let std_num: Option<rust_decimal::Decimal> =
                            r.get("standard_numeric_value");
                        let unit: Option<String> = r.get("standard_unit_reference");
                        let dt: String = r.get("response_datatype");

                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                            csv_escape(&comp),
                            csv_escape(&cin),
                            csv_escape(&sec),
                            csv_escape(&s),
                            p,
                            csv_escape(&ind_type),
                            csv_escape(&grp),
                            csv_escape(&tag),
                            csv_escape(&ctx),
                            csv_escape(&raw.unwrap_or_default()),
                            csv_escape(
                                &std_num
                                    .map(|n| n.normalize().to_string())
                                    .unwrap_or_default()
                            ),
                            csv_escape(&unit.unwrap_or_default()),
                            csv_escape(&dt),
                        )?;
                    }
                    writer.flush()
                })();

                match write_res {
                    Ok(_) => (
                        format!(
                            "✓ Successfully exported {} facts to {}",
                            count,
                            dest_path.display()
                        ),
                        false,
                    ),
                    Err(e) => (format!("Failed to write export file: {}", e), true),
                }
            }
            Err(e) => (format!("Export query failed: {}", e), true),
        };

        let _ = export_tx.send(feedback).await;
        flag.store(false, Ordering::Relaxed);
    });
}

fn trigger_db_backup(state: &mut AppState, target_path: PathBuf) {
    let flag = state.export_state.is_dumping_db.clone();
    flag.store(true, Ordering::Relaxed);

    let db_name = state.config.db_name.clone();
    let db_user = state.config.db_user.clone();
    let db_host = state.config.db_host.clone();
    let db_port = state.config.db_port;
    let db_password = state.db_password_input.clone();
    let container_name = state.config.db_container_name.clone();
    let use_container = crate::system::container::is_container_running(&container_name);
    let export_tx = state.export_tx.clone();

    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();

        let out_file = match File::create(&target_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = rt.block_on(
                    export_tx.send((format!("Failed to create backup file: {}", e), true)),
                );
                flag.store(false, Ordering::Relaxed);
                return;
            }
        };

        let output = if use_container {
            let runtime = if Command::new("podman")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
            {
                "podman"
            } else {
                "docker"
            };
            // Forward PGPASSWORD via host process env without exposing secret in CLI args
            Command::new(runtime)
                .env("PGPASSWORD", &db_password)
                .args([
                    "exec",
                    "-e",
                    "PGPASSWORD",
                    &container_name,
                    "pg_dump",
                    "-U",
                    &db_user,
                    "-d",
                    &db_name,
                    "-F",
                    "c",
                ])
                .stdout(Stdio::from(out_file))
                .stderr(Stdio::piped())
                .output()
        } else {
            Command::new("pg_dump")
                .env("PGPASSWORD", &db_password)
                .args([
                    "-h",
                    &db_host,
                    "-p",
                    &db_port.to_string(),
                    "-U",
                    &db_user,
                    "-d",
                    &db_name,
                    "-F",
                    "c",
                ])
                .stdout(Stdio::from(out_file))
                .stderr(Stdio::piped())
                .output()
        };

        let feedback = match output {
            Ok(out) if out.status.success() => (
                format!("✓ Database backup saved: {}", target_path.display()),
                false,
            ),
            Ok(out) => {
                let _ = std::fs::remove_file(&target_path);
                let err = String::from_utf8_lossy(&out.stderr);
                (format!("pg_dump failed: {}", err.trim()), true)
            }
            Err(e) => {
                let _ = std::fs::remove_file(&target_path);
                (format!("Failed to execute pg_dump: {}", e), true)
            }
        };

        let _ = rt.block_on(export_tx.send(feedback));
        flag.store(false, Ordering::Relaxed);
    });
}
