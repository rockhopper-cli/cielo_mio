// src/downloader/engine.rs

use crate::downloader::db::DatabaseManager;
use crate::downloader::streaming::download_file_atomic;
use crate::downloader::types::{
    DownloadConfig, DownloadEvent, FileSyncStatus, FileType, Market, 
};
use chrono::{Datelike, Local, NaiveDate};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

/// Generates sliding <= 1-year windows from start_date to end_date
pub fn generate_date_windows(start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
    let mut windows = Vec::new();
    let mut cur = start;

    while cur <= end {
        let next_year = cur.year() + 1;
        let nxt_candidate = NaiveDate::from_ymd_opt(next_year, cur.month(), cur.day())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(next_year, cur.month(), 28).unwrap())
            - chrono::Duration::days(1);

        let nxt = nxt_candidate.min(end);
        windows.push((cur, nxt));
        cur = nxt + chrono::Duration::days(1);
    }

    windows
}

pub struct ExchangeDownloader {
    db: DatabaseManager,
}

impl ExchangeDownloader {
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    /// Orchestrates simultaneous dual-market download (BSE + NSE in parallel)
    pub async fn run(
        &self,
        config: &DownloadConfig,
        event_tx: Sender<DownloadEvent>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        // Enforce half-split and increment odd test numbers by 1
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

        // Run both market downloads concurrently using tokio::join!
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

    /// Internal runner for a single exchange market
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
                let c = super::scrapers::build_nse_client()?;
                super::scrapers::warmup_nse(&c)
                    .await
                    .map_err(|e| format!("NSE warmup failed: {}", e))?;
                let _ = event_tx
                    .send(DownloadEvent::Log(
                        "✓ NSE session warmup complete.".to_string(),
                    ))
                    .await;
                c
            }
            Market::Bse => super::scrapers::build_bse_client()?,
        };

        // Preserves distinct directory separation for each market
        let market_root = config.output_dir.join(exchange_name.to_lowercase());
        let pdf_dir = market_root.join("pdf");
        let xml_dir = market_root.join("xml");

        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = Local::now().date_naive();
        let mut windows = generate_date_windows(start_date, end_date);
        windows.reverse(); // Latest filings first

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
                Market::Bse => {
                    super::scrapers::fetch_bse_records(&client, from_d, to_d, &config).await
                }
                Market::Nse => {
                    super::scrapers::fetch_nse_records(&client, from_d, to_d, &config, &event_tx)
                        .await
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

                // Database duplicate check
                if self
                    .db
                    .check_file_status(&pending.file_name)
                    .await
                    .unwrap_or(FileSyncStatus::NewFile)
                    == FileSyncStatus::AlreadyMapped
                {
                    skipped += 1;
                    let _ = event_tx
                        .send(DownloadEvent::FileSkipped {
                            file_name: pending.file_name.clone(),
                            reason: format!("Already mapped in DB ({})", exchange_name),
                        })
                        .await;
                    continue;
                }

                let target_path = match pending.file_type {
                    FileType::Pdf => pdf_dir.join(&pending.file_name),
                    FileType::Xml => xml_dir.join(&pending.file_name),
                };

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
                        let _ = self.db.record_success(&pending).await;
                        let _ = event_tx
                            .send(DownloadEvent::FileDownloaded {
                                file_name: pending.file_name.clone(),
                                path: saved_path,
                            })
                            .await;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let _ = self.db.record_failure(&pending, &err_str).await;
                        let _ = event_tx
                            .send(DownloadEvent::FileFailed {
                                file_name: pending.file_name.clone(),
                                error: format!("[{}] {}", exchange_name, err_str),
                            })
                            .await;
                    }
                }

                if let Some(max) = config.max_files {
                    if downloaded >= max {
                        let _ = event_tx
                            .send(DownloadEvent::Log(format!(
                                "[{}] Reached target limit of {} files.",
                                exchange_name, max
                            )))
                            .await;
                        break;
                    }
                }
            }

            if let Some(max) = config.max_files {
                if downloaded >= max {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        Ok((downloaded, skipped))
    }
}
