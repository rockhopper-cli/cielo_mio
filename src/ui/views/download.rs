// src/ui/views/download.rs

use crate::domain::download::{DownloadConfig, DownloadEvent, Market};
use crate::domain::taxonomy::IndustryRegistry;
use crate::domain::units::UnitRegistry;
use crate::pipeline::ingestion::IngestionManager;
use crate::pipeline::xbrl::XbrlParser;
use crate::pipeline::xbrl::transform::FormDictionary;
use crate::ui::state::AppState;

use eframe::egui::{self, Color32, Frame, Margin, RichText, ScrollArea, Ui};
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("📥 Exchange Downloader & Ingestion Pipeline");
    ui.add_space(6.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(2, 2))
                .show(ui, |ui| {
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Target Exchanges:").strong());
                            Frame::NONE
                                .fill(Color32::from_rgb(32, 40, 56))
                                .corner_radius(4)
                                .inner_margin(Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("⚡ Simultaneous Dual Sync (BSE & NSE)")
                                            .strong()
                                            .color(Color32::from_rgb(100, 180, 240)),
                                    );
                                });

                            ui.separator();

                            ui.checkbox(&mut state.download_xml, "XML Reports");
                            ui.checkbox(&mut state.download_pdf, "PDF Filings");

                            ui.separator();

                            ui.checkbox(&mut state.test_mode, "Test Limit:");
                            if state.test_mode {
                                ui.add(
                                    egui::DragValue::new(&mut state.test_cap)
                                        .range(2..=100)
                                        .suffix(" files total"),
                                );
                            }
                        });

                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(
                                &mut state.auto_ingest,
                                RichText::new("⚡ Automatically parse & ingest XML filings into database upon download completion")
                                    .color(Color32::from_rgb(110, 200, 240)),
                            );
                        });

                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Target Folder:").strong());
                            ui.monospace(state.output_dir.display().to_string());
                            if ui.button("Browse...").clicked()
                                && let Some(folder) = rfd::FileDialog::new().pick_folder()
                            {
                                state.output_dir = folder;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        let is_running = state.is_downloading.load(Ordering::Relaxed)
                            || state.is_ingesting.load(Ordering::Relaxed);

                        if !is_running {
                            if ui
                                .button(
                                    RichText::new("▶ Start Exchange Download")
                                        .strong()
                                        .color(Color32::from_rgb(80, 210, 120)),
                                )
                                .clicked()
                            {
                                start_download_task(state);
                            }

                            if ui
                                .button(
                                    RichText::new("⚡ Ingest Unmapped XML Files Manually")
                                        .strong()
                                        .color(Color32::from_rgb(100, 180, 240)),
                                )
                                .clicked()
                            {
                                start_ingestion_task(state);
                            }

                            if ui
                                .button(
                                    RichText::new("🔄 Reset Failed Filings")
                                        .small()
                                        .color(Color32::from_rgb(220, 180, 90)),
                                )
                                .on_hover_text(
                                    "Clear error status on unmapped files so they can be re-attempted",
                                )
                                .clicked()
                            {
                                reset_failed_filings(state);
                            }
                        } else {
                            if ui
                                .button(
                                    RichText::new("⏹ Stop Active Task")
                                        .strong()
                                        .color(Color32::from_rgb(240, 80, 80)),
                                )
                                .clicked()
                                && let Some(ref token) = state.cancel_token
                            {
                                token.cancel();
                                state.add_log(
                                    "[USER] Sent stop signal to active pipeline.".to_string(),
                                );
                            }
                            ui.spinner();
                            if state.is_downloading.load(Ordering::Relaxed) {
                                ui.label("Downloading filings from exchange in background...");
                            } else if state.is_ingesting.load(Ordering::Relaxed) {
                                ui.label(
                                    "Parsing XBRL and ingesting disclosures into PostgreSQL...",
                                );
                            }
                        }
                    });

                    ui.add_space(12.0);

                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Console Drawer:").strong());
                            let toggle_label = if state.show_logs {
                                "▼ Hide Console"
                            } else {
                                "▲ Show Live Activity"
                            };

                            if ui.button(toggle_label).clicked() {
                                state.show_logs = !state.show_logs;
                            }

                            if let Some(last) = state.log_messages.back() {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("Latest: {}", last))
                                            .small()
                                            .color(Color32::from_rgb(180, 190, 205)),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                    });
                });
        });
}

fn start_download_task(state: &mut AppState) {
    if !state.download_xml && !state.download_pdf {
        state.add_log(
            "[WARN] Select at least one file type (XML Reports or PDF Filings) before downloading."
                .to_string(),
        );
        return;
    }

    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            state.add_log("[ERROR] Cannot start download: Database not connected!".to_string());
            return;
        }
    };

    let config = DownloadConfig {
        market: Market::Bse,
        output_dir: state.output_dir.clone(),
        download_pdf: state.download_pdf,
        download_xml: state.download_xml,
        max_files: if state.test_mode {
            Some(state.test_cap)
        } else {
            None
        },
    };

    let auto_ingest = state.auto_ingest;
    let output_dir = state.output_dir.clone();

    let cancel_token = CancellationToken::new();
    state.cancel_token = Some(cancel_token.clone());
    let is_downloading = state.is_downloading.clone();
    let is_ingesting = state.is_ingesting.clone();
    is_downloading.store(true, Ordering::Relaxed);

    let event_tx = state.event_tx.clone();

    tokio::spawn(async move {
        let downloader = crate::pipeline::scrapers::ExchangeDownloader::new(pool.clone());
        let download_result = downloader
            .run(&config, event_tx.clone(), cancel_token.clone())
            .await;

        is_downloading.store(false, Ordering::Relaxed);

        if let Err(e) = download_result {
            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[ERROR] Download task halted: {}",
                    e
                )))
                .await;
            return;
        }

        if auto_ingest && !cancel_token.is_cancelled() {
            let _ = event_tx
                .send(DownloadEvent::Log(
                    "Auto-ingest enabled: Starting XBRL parsing & database mapping...".to_string(),
                ))
                .await;
            is_ingesting.store(true, Ordering::Relaxed);

            let _ = execute_ingestion(pool, output_dir, event_tx.clone(), cancel_token).await;

            is_ingesting.store(false, Ordering::Relaxed);
        }
    });
}

fn start_ingestion_task(state: &mut AppState) {
    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            state.add_log("[ERROR] Cannot start ingestion: Database not connected!".to_string());
            return;
        }
    };

    let is_ingesting = state.is_ingesting.clone();
    is_ingesting.store(true, Ordering::Relaxed);
    let event_tx = state.event_tx.clone();
    let output_dir = state.output_dir.clone();

    let cancel_token = CancellationToken::new();
    state.cancel_token = Some(cancel_token.clone());

    tokio::spawn(async move {
        let _ = execute_ingestion(pool, output_dir, event_tx, cancel_token).await;
        is_ingesting.store(false, Ordering::Relaxed);
    });
}

fn reset_failed_filings(state: &mut AppState) {
    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            state.add_log(
                "[ERROR] Cannot reset failed filings: Database not connected!".to_string(),
            );
            return;
        }
    };
    let event_tx = state.event_tx.clone();

    tokio::spawn(async move {
        match crate::storage::repository::files::reset_failed_filings(&pool).await {
            Ok(affected) => {
                let _ = event_tx
                    .send(DownloadEvent::Log(format!(
                        "[RESET] Reset error status on {} failed filings. Ready for re-ingestion.",
                        affected
                    )))
                    .await;
            }
            Err(e) => {
                let _ = event_tx
                    .send(DownloadEvent::Log(format!(
                        "[ERROR] Failed to reset unmapped files: {}",
                        e
                    )))
                    .await;
            }
        }
    });
}

pub async fn execute_ingestion(
    pool: PgPool,
    output_dir: PathBuf,
    event_tx: Sender<DownloadEvent>,
    cancel_token: CancellationToken,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let registry = match UnitRegistry::load_from_db(&pool).await {
        Ok(r) => r,
        Err(e) => {
            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[ERROR] Failed to load unit conversion rules: {}",
                    e
                )))
                .await;
            return Err(e.into());
        }
    };

    let dictionary = match FormDictionary::load_from_db(&pool).await {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[ERROR] Failed to load taxonomy dictionary (xbrl_tag_rules): {}",
                    e
                )))
                .await;
            return Err(e.into());
        }
    };

    let industry_registry = match IndustryRegistry::load_from_db(&pool).await {
        Ok(r) => r,
        Err(e) => {
            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[ERROR] Failed to load industry master (industry_master): {}",
                    e
                )))
                .await;
            return Err(e.into());
        }
    };

    let parser = XbrlParser::new(&registry);
    let ingestion = IngestionManager::new(pool.clone(), industry_registry);

    let batch_size: i64 = 50;
    let mut total_mapped = 0;

    let pending_count = crate::storage::repository::files::count_pending_xml_filings(&pool)
        .await
        .unwrap_or(0);

    let _ = event_tx
        .send(DownloadEvent::Log(format!(
            "Found {} unmapped XML filings ready for ingestion.",
            pending_count
        )))
        .await;

    if pending_count == 0 {
        return Ok(0);
    }

    loop {
        if cancel_token.is_cancelled() {
            let _ = event_tx
                .send(DownloadEvent::Log(
                    "[USER] Ingestion task cancelled by user.".to_string(),
                ))
                .await;
            break;
        }

        let batch =
            crate::storage::repository::files::fetch_pending_xml_batch(&pool, batch_size).await?;

        if batch.is_empty() {
            break;
        }

        for (file_id, file_name, stock_market, file_symbol, file_scrip) in batch {
            if cancel_token.is_cancelled() {
                break;
            }

            let candidate_paths = [
                output_dir
                    .join(stock_market.to_lowercase())
                    .join("xml")
                    .join(&file_name),
                output_dir
                    .join(stock_market.to_uppercase())
                    .join("xml")
                    .join(&file_name),
                output_dir.join("xml").join(&file_name),
                output_dir.join(&file_name),
            ];

            let disk_path = match candidate_paths.into_iter().find(|p| p.exists()) {
                Some(p) => p,
                None => {
                    let err_msg = "XML filing not found on disk".to_string();
                    ingestion.record_failure(file_id, &err_msg).await;
                    let _ = event_tx
                        .send(DownloadEvent::Log(format!(
                            "[WARN] XML filing not found on disk: {} (Marked as failed)",
                            file_name
                        )))
                        .await;
                    continue;
                }
            };

            let parsed_and_enriched = tokio::task::block_in_place(|| {
                parser.parse_file(&disk_path).map(|filing| {
                    dictionary.enrich_facts(&filing, file_symbol.as_deref(), file_scrip.as_deref())
                })
            });

            match parsed_and_enriched {
                Ok((enriched, profile)) => {
                    if profile.corporate_id_number.trim().is_empty() || enriched.is_empty() {
                        let err_msg = "Blank File: No valid entity identifier or BRSR facts found"
                            .to_string();
                        ingestion.record_failure(file_id, &err_msg).await;
                        let _ = event_tx
                            .send(DownloadEvent::Log(format!(
                                "[WARN] Skipped empty/invalid XML {}: {}",
                                file_name, err_msg
                            )))
                            .await;
                        continue;
                    }

                    let fact_count = enriched.len();
                    let cin = profile.corporate_id_number.clone();
                    let comp_name = profile.company_name.clone();

                    match ingestion
                        .ingest_filing(file_id, &stock_market, &profile, &enriched)
                        .await
                    {
                        Ok(entity_id) => {
                            total_mapped += 1;
                            let _ = event_tx
                                .send(DownloadEvent::Log(format!(
                                    "[SUCCESS] Ingested {} (CIN: {}) - {} facts mapped (UUID: {})",
                                    comp_name, cin, fact_count, entity_id
                                )))
                                .await;
                        }
                        Err(e) => {
                            let err_msg = format!("Database write error: {}", e);
                            ingestion.record_failure(file_id, &err_msg).await;
                            let _ = event_tx
                                .send(DownloadEvent::Log(format!(
                                    "[ERROR] Failed to ingest {}: {}",
                                    file_name, err_msg
                                )))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("XBRL parse error: {}", e);
                    ingestion.record_failure(file_id, &err_msg).await;
                    let _ = event_tx
                        .send(DownloadEvent::Log(format!(
                            "[ERROR] Parse failed for {}: {}",
                            file_name, err_msg
                        )))
                        .await;
                }
            }
        }
    }

    let _ = event_tx
        .send(DownloadEvent::Log(format!(
            "=== Ingestion Complete: Successfully mapped {} filings into database ===",
            total_mapped
        )))
        .await;

    Ok(total_mapped)
}
