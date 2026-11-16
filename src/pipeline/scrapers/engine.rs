// src/pipeline/scrapers/engine.rs

use super::scraper;
use super::streaming::download_file_atomic;
use crate::domain::download::{
    DownloadConfig, DownloadEvent, FileSyncStatus, FileType, Market, generate_date_windows,
};
use crate::storage::repository::files;
use chrono::{Local, NaiveDate};
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

pub struct ExchangeDownloader {
    pool: PgPool,
}

impl ExchangeDownloader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Orchestrates simultaneous dual-market download (BSE + NSE in parallel)
    pub async fn run(
        &self,
        config: &DownloadConfig,
        event_tx: Sender<DownloadEvent>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        let per_market_cap = match config.max_files {
            Some(cap) => {
                let adjusted = if cap % 2 != 0 { cap + 1 } else { cap };
                let half = adjusted / 2;
                let _ = event_tx
                    .send(DownloadEvent::Log(format!(
                        "[CONFIG] Test cap: {} files ({} BSE + {} NSE).",
                        adjusted, half, half
                    )))
                    .await;
                Some(half)
            }
            None => None,
        };

        let mut bse_config = config.clone();
        bse_config.market = Market::Bse;
        bse_config.max_files = per_market_cap;

        let mut nse_config = config.clone();
        nse_config.market = Market::Nse;
        nse_config.max_files = per_market_cap;

        let _ = event_tx
            .send(DownloadEvent::Log(
                "🚀 Launching simultaneous dual-exchange download (BSE & NSE)...".to_string(),
            ))
            .await;

        let bse_task = self.run_single_market(
            Market::Bse,
            bse_config,
            event_tx.clone(),
            cancel_token.clone(),
        );

        let nse_task = self.run_single_market(
            Market::Nse,
            nse_config,
            event_tx.clone(),
            cancel_token.clone(),
        );

        let (bse_res, nse_res) = tokio::join!(bse_task, nse_task);

        let (bse_dl, bse_sk) = match bse_res {
            Ok(counts) => counts,
            Err(e) => {
                let _ = event_tx
                    .send(DownloadEvent::Log(format!(
                        "[ERROR] BSE download halted: {}",
                        e
                    )))
                    .await;
                (0, 0)
            }
        };

        let (nse_dl, nse_sk) = match nse_res {
            Ok(counts) => counts,
            Err(e) => {
                let _ = event_tx
                    .send(DownloadEvent::Log(format!(
                        "[ERROR] NSE download halted: {}",
                        e
                    )))
                    .await;
                (0, 0)
            }
        };

        let total_downloaded = bse_dl + nse_dl;
        let total_skipped = bse_sk + nse_sk;

        let _ = event_tx
            .send(DownloadEvent::Done {
                total_downloaded,
                total_skipped,
            })
            .await;

        Ok(())
    }

    async fn run_single_market(
        &self,
        market: Market,
        config: DownloadConfig,
        event_tx: Sender<DownloadEvent>,
        cancel_token: CancellationToken,
    ) -> Result<(usize, usize), String> {
        let exchange_name = market.as_str();

        let client = match market {
            Market::Nse => {
                let c = scraper::build_nse_client()?;
                scraper::warmup_nse(&c)
                    .await
                    .map_err(|e| format!("NSE warmup failed: {}", e))?;
                let _ = event_tx
                    .send(DownloadEvent::Log(
                        "✓ NSE session warmup complete.".to_string(),
                    ))
                    .await;
                c
            }
            Market::Bse => scraper::build_bse_client()?,
        };

        let market_root = config.output_dir.join(exchange_name.to_lowercase());
        let pdf_dir = market_root.join("pdf");
        let xml_dir = market_root.join("xml");

        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = Local::now().date_naive();
        let mut windows = generate_date_windows(start_date, end_date);
        windows.reverse();

        let mut downloaded = 0usize;
        let mut skipped = 0usize;

        for (from_d, to_d) in windows {
            if cancel_token.is_cancelled() {
                break;
            }

            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[{}] Querying window: {} to {}",
                    exchange_name, from_d, to_d
                )))
                .await;

            let records = match market {
                Market::Bse => scraper::fetch_bse_records(&client, from_d, to_d, &config).await,
                Market::Nse => {
                    scraper::fetch_nse_records(&client, from_d, to_d, &config, &event_tx).await
                }
            };

            let records = match records {
                Ok(recs) => recs,
                Err(e) => {
                    let _ = event_tx
                        .send(DownloadEvent::Log(format!(
                            "[{}] Query error: {}",
                            exchange_name, e
                        )))
                        .await;
                    continue;
                }
            };

            let _ = event_tx
                .send(DownloadEvent::Log(format!(
                    "[{}] Found {} matching files in window.",
                    exchange_name,
                    records.len()
                )))
                .await;

            for pending in records {
                if cancel_token.is_cancelled() {
                    break;
                }
                if let Some(max) = config.max_files
                    && downloaded >= max
                {
                    break;
                }

                let target_path = match pending.file_type {
                    FileType::Pdf => pdf_dir.join(&pending.file_name),
                    FileType::Xml => xml_dir.join(&pending.file_name),
                };

                let status = files::check_file_status(&self.pool, &pending.file_name)
                    .await
                    .unwrap_or(FileSyncStatus::NewFile);

                if status == FileSyncStatus::AlreadyMapped {
                    skipped += 1;
                    let _ = event_tx
                        .send(DownloadEvent::FileSkipped {
                            file_name: pending.file_name.clone(),
                            reason: format!("Already mapped in DB ({})", exchange_name),
                        })
                        .await;
                    continue;
                }

                let already_on_disk = target_path.exists()
                    && tokio::fs::metadata(&target_path)
                        .await
                        .is_ok_and(|m| m.len() > 0);

                if already_on_disk {
                    if matches!(status, FileSyncStatus::NewFile) {
                        let _ = files::record_file_success(&self.pool, &pending).await;
                    }
                    skipped += 1;
                    let _ = event_tx
                        .send(DownloadEvent::FileSkipped {
                            file_name: pending.file_name.clone(),
                            reason: format!("Already on disk ({})", exchange_name),
                        })
                        .await;
                    continue;
                }

                match download_file_atomic(
                    &client,
                    &pending.download_url,
                    &target_path,
                    &cancel_token,
                )
                .await
                {
                    Ok(saved_path) => {
                        downloaded += 1;
                        let _ = files::record_file_success(&self.pool, &pending).await;
                        let _ = event_tx
                            .send(DownloadEvent::FileDownloaded {
                                file_name: pending.file_name.clone(),
                                path: saved_path,
                            })
                            .await;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let _ = files::record_file_failure(&self.pool, &pending, &err_str).await;
                        let _ = event_tx
                            .send(DownloadEvent::FileFailed {
                                file_name: pending.file_name.clone(),
                                error: format!("[{}] {}", exchange_name, err_str),
                            })
                            .await;
                    }
                }
            }

            if let Some(max) = config.max_files
                && downloaded >= max
            {
                break;
            }

            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        Ok((downloaded, skipped))
    }
}
