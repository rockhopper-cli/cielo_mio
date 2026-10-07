// src/downloader/scrapers.rs

use crate::downloader::filename::{parse_date_time_from_filename, sanitize_filename};
use crate::downloader::types::{DownloadConfig, DownloadEvent, FileType, Market, PendingFile};
use chrono::{Datelike, NaiveDate};
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderValue, REFERER, USER_AGENT,
};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

const BSE_API_URL: &str = "https://api.bseindia.com/BseIndiaAPI/api/GetCorXbrlDetails_ng/w";
const NSE_BASE_URL: &str = "https://www.nseindia.com";
const NSE_LIST_URL: &str = "https://www.nseindia.com/companies-listing/corporate-filings-bussiness-sustainabilitiy-reports";
const NSE_API_URL: &str = "https://www.nseindia.com/api/corporate-bussiness-sustainabilitiy";

// ============================================================================
// 1. BSE Client & Scraper
// ============================================================================

pub fn build_bse_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(REFERER, HeaderValue::from_static("https://www.bseindia.com/"));
    headers.insert("Origin", HeaderValue::from_static("https://www.bseindia.com"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-site"));
    headers.insert("Sec-GPC", HeaderValue::from_static("1"));

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn fetch_bse_records(
    client: &Client,
    from_d: NaiveDate,
    to_d: NaiveDate,
    config: &DownloadConfig,
) -> Result<Vec<PendingFile>, String> {
    let from_str = format!("{}/{}/{}", from_d.year(), from_d.month(), from_d.day());
    let to_str = format!("{}/{}/{}", to_d.year(), to_d.month(), to_d.day());

    let query_params = [
        ("Flag", "43"),
        ("scripcode", ""),
        ("fromdate", &from_str),
        ("todate", &to_str),
    ];

    let resp = client.get(BSE_API_URL).query(&query_params).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("BSE returned status {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = json.as_array().cloned()
        .or_else(|| json.get("Table").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    let mut files = Vec::new();

    for rec in &items {
        let company = rec.get("scrip_name").or_else(|| rec.get("COMPANY")).and_then(|v| v.as_str()).unwrap_or("company");
        let scrip_code = rec.get("scrip_cd").or_else(|| rec.get("scripcode")).and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string()))).unwrap_or_default();
        let date_str = rec.get("xbrl_submission_date").or_else(|| rec.get("DT_TM")).and_then(|v| v.as_str()).unwrap_or("");
        let (upload_date, upload_time) = parse_date_time_from_filename(date_str);

        if let Some(obj) = rec.as_object() {
            for (key, val) in obj {
                if let Some(s) = val.as_str() {
                    let lo = s.to_lowercase();
                    if config.download_pdf && (lo.ends_with(".pdf") || lo.contains(".pdf?")) {
                        let url = if s.starts_with("http") { s.to_string() } else { format!("https://www.bseindia.com/xml-data/corpfiling/AttachLive/{}", s) };
                        files.push(PendingFile {
                            file_name: format!("{}_{}_BRSR_{}.pdf", sanitize_filename(company), scrip_code, sanitize_filename(s.trim_end_matches(".pdf"))),
                            download_url: url,
                            stock_market: Market::Bse,
                            file_type: FileType::Pdf,
                            upload_date,
                            upload_time,
                        });
                    } else if config.download_xml && (lo.ends_with(".xml") || lo.contains(".xml?")) {
                        let url = if s.starts_with("http") { s.to_string() } else { format!("https://www.bseindia.com/{}", s.trim_start_matches('/')) };
                        files.push(PendingFile {
                            file_name: format!("{}_{}_BRSR_XML_{}.xml", sanitize_filename(company), scrip_code, sanitize_filename(key)),
                            download_url: url,
                            stock_market: Market::Bse,
                            file_type: FileType::Xml,
                            upload_date,
                            upload_time,
                        });
                    }
                }
            }
        }

        if config.download_xml && !files.iter().any(|f| f.file_type == FileType::Xml) {
            if let Some(news_id) = rec.get("NEWSID").and_then(|v| v.as_str()) {
                if !scrip_code.is_empty() {
                    files.push(PendingFile {
                        file_name: format!("{}_{}_BRSR_XML_{}.xml", sanitize_filename(company), scrip_code, sanitize_filename(news_id)),
                        download_url: format!("https://www.bseindia.com/Msource/90D/CorpXbrlGen.aspx?Bsenewid={}&Scripcode={}", news_id, scrip_code),
                        stock_market: Market::Bse,
                        file_type: FileType::Xml,
                        upload_date,
                        upload_time,
                    });
                }
            }
        }
    }

    files.dedup_by(|a, b| a.download_url == b.download_url);
    Ok(files)
}

// ============================================================================
// 2. NSE Client & Scraper
// ============================================================================

pub fn build_nse_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/122.0.0.0 Safari/537.36"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/javascript, */*; q=0.01"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(REFERER, HeaderValue::from_static(NSE_LIST_URL));
    headers.insert("X-Requested-With", HeaderValue::from_static("XMLHttpRequest"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

    Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn warmup_nse(client: &Client) -> Result<(), reqwest::Error> {
    let _ = client.get(NSE_BASE_URL).send().await?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let _ = client.get(NSE_LIST_URL).send().await?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    Ok(())
}

pub async fn fetch_nse_records(
    client: &Client,
    from_d: NaiveDate,
    to_d: NaiveDate,
    config: &DownloadConfig,
    event_tx: &Sender<DownloadEvent>,
) -> Result<Vec<PendingFile>, String> {
    let from_str = from_d.format("%d-%m-%Y").to_string();
    let to_str = to_d.format("%d-%m-%Y").to_string();
    let query_url = format!("{}?index=equities&from_date={}&to_date={}", NSE_API_URL, from_str, to_str);

    let mut resp = client.get(&query_url).send().await.map_err(|e| e.to_string())?;

    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        let _ = event_tx.send(DownloadEvent::Log("[WARN] 403 Forbidden. Refreshing session...".to_string())).await;
        let _ = warmup_nse(client).await;
        resp = client.get(&query_url).send().await.map_err(|e| e.to_string())?;
    }

    if !resp.status().is_success() {
        return Err(format!("NSE returned status {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = json.as_array().cloned()
        .or_else(|| json.get("data").or_else(|| json.get("rows")).and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    let mut files = Vec::new();

    for rec in &items {
        let company = rec.get("company").or_else(|| rec.get("companyName")).or_else(|| rec.get("symbol")).and_then(|v| v.as_str()).unwrap_or("company");
        let fy_from = rec.get("from_year").or_else(|| rec.get("fromYear")).and_then(|v| v.as_str()).unwrap_or("");
        let fy_to = rec.get("to_year").or_else(|| rec.get("toYear")).and_then(|v| v.as_str()).unwrap_or("");
        let prefix = format!("{}_{}_{}", sanitize_filename(company), fy_from, fy_to).trim_matches('_').to_string();

        if let Some(obj) = rec.as_object() {
            for val in obj.values() {
                if let Some(raw_url) = val.as_str() {
                    let lo = raw_url.to_lowercase();
                    let is_pdf = lo.ends_with(".pdf") || lo.contains(".pdf?");
                    let is_xml = lo.ends_with(".xml") || lo.contains(".xml?") || lo.ends_with(".xbrl");

                    let (is_match, file_type) = if config.download_pdf && is_pdf {
                        (true, FileType::Pdf)
                    } else if config.download_xml && is_xml {
                        (true, FileType::Xml)
                    } else {
                        (false, FileType::Pdf)
                    };

                    if is_match {
                        let full_url = if raw_url.starts_with("http") {
                            raw_url.to_string()
                        } else if raw_url.starts_with('/') {
                            format!("https://nsearchives.nseindia.com{}", raw_url)
                        } else {
                            format!("https://nsearchives.nseindia.com/corporate/{}", raw_url)
                        };

                        let raw_name = full_url.split('?').next().and_then(|u| u.split('/').last()).unwrap_or("report");
                        let file_name = if !prefix.is_empty() { format!("{}_{}", prefix, raw_name) } else { raw_name.to_string() };
                        let (upload_date, upload_time) = parse_date_time_from_filename(&file_name);

                        files.push(PendingFile {
                            file_name,
                            download_url: full_url,
                            stock_market: Market::Nse,
                            file_type,
                            upload_date,
                            upload_time,
                        });
                    }
                }
            }
        }
    }

    files.dedup_by(|a, b| a.download_url == b.download_url);
    Ok(files)
}