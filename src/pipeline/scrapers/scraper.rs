// src/pipeline/scrapers/scraper.rs

use crate::domain::download::{
    DownloadConfig, DownloadEvent, FileType, Market, PendingFile, parse_date_time_from_filename,
    sanitize_filename,
};
use chrono::NaiveDate;
use reqwest::Client;
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderValue, REFERER, USER_AGENT,
};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

const BSE_API_URL: &str = "https://api.bseindia.com/BseIndiaAPI/api/GetCorXbrlDetails_ng/w";
const NSE_BASE_URL: &str = "https://www.nseindia.com";
const NSE_LIST_URL: &str = "https://www.nseindia.com/companies-listing/corporate-filings-bussiness-sustainabilitiy-reports";
const NSE_API_URL: &str = "https://www.nseindia.com/api/corporate-bussiness-sustainabilitiy";

/// Truncates `prefix` before `unique_suffix` so unique filing IDs are never clipped off long company names.
fn build_safe_filename(
    prefix: &str,
    unique_suffix: &str,
    ext: &str,
    max_total_len: usize,
) -> String {
    let max_stem = max_total_len.saturating_sub(ext.len() + 1);
    let clean_suffix = unique_suffix.trim_matches('_');
    let clean_prefix = prefix.trim_matches('_');

    let mut stem = if clean_suffix.is_empty() {
        clean_prefix.chars().take(max_stem).collect::<String>()
    } else if clean_prefix.is_empty() {
        let len = clean_suffix.chars().count();
        clean_suffix
            .chars()
            .skip(len.saturating_sub(max_stem))
            .collect()
    } else {
        let suffix_len = clean_suffix.chars().count();
        if suffix_len + 1 >= max_stem {
            clean_suffix
                .chars()
                .skip(suffix_len.saturating_sub(max_stem))
                .collect()
        } else {
            let prefix_budget = max_stem - suffix_len - 1;
            let clipped_prefix: String = clean_prefix
                .chars()
                .take(prefix_budget)
                .collect::<String>()
                .trim_end_matches('_')
                .to_string();
            if clipped_prefix.is_empty() {
                clean_suffix.to_string()
            } else {
                format!("{}_{}", clipped_prefix, clean_suffix)
            }
        }
    };

    stem = stem.trim_matches('_').to_string();
    if stem.is_empty() {
        stem = "filing".to_string();
    }
    format!("{}.{}", stem, ext)
}

pub fn build_bse_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (X11; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.bseindia.com/"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://www.bseindia.com"),
    );
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
    let from_str = from_d.format("%Y/%m/%d").to_string();
    let to_str = to_d.format("%Y/%m/%d").to_string();

    let query_params = [
        ("Flag", "43"),
        ("scripcode", ""),
        ("fromdate", &from_str),
        ("todate", &to_str),
    ];

    let resp = client
        .get(BSE_API_URL)
        .query(&query_params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("BSE returned status {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = json
        .as_array()
        .cloned()
        .or_else(|| json.get("Table").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    let mut files = Vec::new();

    for rec in &items {
        let company = rec
            .get("scrip_name")
            .or_else(|| rec.get("COMPANY"))
            .and_then(|v| v.as_str())
            .unwrap_or("company");
        let scrip_code = rec
            .get("scrip_cd")
            .or_else(|| rec.get("scripcode"))
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            })
            .unwrap_or_default();
        let scrip_code_opt = if scrip_code.trim().is_empty() {
            None
        } else {
            Some(scrip_code.trim().to_string())
        };
        let news_id = rec
            .get("NEWSID")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let date_str = rec
            .get("xbrl_submission_date")
            .or_else(|| rec.get("DT_TM"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let (upload_date, upload_time) = parse_date_time_from_filename(date_str);

        let mut rec_has_xml = false;
        let date_tag = upload_date.format("%Y%m%d").to_string();
        let unique_suffix = news_id
            .map(sanitize_filename)
            .unwrap_or_else(|| date_tag.clone());

        if let Some(obj) = rec.as_object() {
            for (key, val) in obj {
                if let Some(s) = val.as_str() {
                    let lo = s.to_lowercase();
                    if config.download_pdf && (lo.ends_with(".pdf") || lo.contains(".pdf?")) {
                        let url = if s.starts_with("http") {
                            s.to_string()
                        } else {
                            format!(
                                "https://www.bseindia.com/xml-data/corpfiling/AttachLive/{}",
                                s.trim_start_matches('/')
                            )
                        };
                        let leaf = s
                            .split('?')
                            .next()
                            .and_then(|p| p.split('/').next_back())
                            .unwrap_or("filing");
                        let stem_leaf = leaf
                            .strip_suffix(".pdf")
                            .or_else(|| leaf.strip_suffix(".PDF"))
                            .unwrap_or(leaf);
                        let prefix = format!(
                            "{}_{}_BRSR",
                            sanitize_filename(company),
                            sanitize_filename(&scrip_code)
                        );
                        files.push(PendingFile {
                            file_name: build_safe_filename(
                                &prefix,
                                &sanitize_filename(stem_leaf),
                                "pdf",
                                220,
                            ),
                            download_url: url,
                            stock_market: Market::Bse,
                            file_type: FileType::Pdf,
                            upload_date,
                            upload_time,
                            symbol: None,
                            scrip_code: scrip_code_opt.clone(),
                        });
                    } else if config.download_xml && (lo.ends_with(".xml") || lo.contains(".xml?"))
                    {
                        rec_has_xml = true;
                        let url = if s.starts_with("http") {
                            s.to_string()
                        } else {
                            format!("https://www.bseindia.com/{}", s.trim_start_matches('/'))
                        };
                        let prefix = format!(
                            "{}_{}_BRSR_XML_{}",
                            sanitize_filename(company),
                            sanitize_filename(&scrip_code),
                            sanitize_filename(key)
                        );
                        files.push(PendingFile {
                            file_name: build_safe_filename(&prefix, &unique_suffix, "xml", 220),
                            download_url: url,
                            stock_market: Market::Bse,
                            file_type: FileType::Xml,
                            upload_date,
                            upload_time,
                            symbol: None,
                            scrip_code: scrip_code_opt.clone(),
                        });
                    }
                }
            }
        }

        if config.download_xml
            && !rec_has_xml
            && let Some(nid) = news_id
            && !scrip_code.trim().is_empty()
        {
            let prefix = format!(
                "{}_{}_BRSR_XML",
                sanitize_filename(company),
                sanitize_filename(&scrip_code)
            );
            files.push(PendingFile {
                file_name: build_safe_filename(&prefix, &sanitize_filename(nid), "xml", 220),
                download_url: format!(
                    "https://www.bseindia.com/Msource/90D/CorpXbrlGen.aspx?Bsenewid={}&Scripcode={}",
                    urlencoding::encode(nid),
                    urlencoding::encode(scrip_code.trim())
                ),
                stock_market: Market::Bse,
                file_type: FileType::Xml,
                upload_date,
                upload_time,
                symbol: None,
                scrip_code: scrip_code_opt.clone(),
            });
        }
    }

    let mut seen_urls = HashSet::with_capacity(files.len());
    files.retain(|f| seen_urls.insert(f.download_url.clone()));
    Ok(files)
}

pub fn build_nse_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/122.0.0.0 Safari/537.36"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/javascript, */*; q=0.01"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(REFERER, HeaderValue::from_static(NSE_LIST_URL));
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_static("XMLHttpRequest"),
    );
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
    let query_url = format!(
        "{}?index=equities&from_date={}&to_date={}",
        NSE_API_URL, from_str, to_str
    );

    let mut resp = client
        .get(&query_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        let _ = event_tx
            .send(DownloadEvent::Log(
                "[WARN] 403 Forbidden. Refreshing session...".to_string(),
            ))
            .await;
        let _ = warmup_nse(client).await;
        resp = client
            .get(&query_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }

    if !resp.status().is_success() {
        return Err(format!("NSE returned status {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let items = json
        .as_array()
        .cloned()
        .or_else(|| {
            json.get("data")
                .or_else(|| json.get("rows"))
                .and_then(|v| v.as_array())
                .cloned()
        })
        .unwrap_or_default();

    let mut files = Vec::new();

    for rec in &items {
        let nse_symbol = rec
            .get("symbol")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());
        let company = rec
            .get("company")
            .or_else(|| rec.get("companyName"))
            .or_else(|| rec.get("symbol"))
            .and_then(|v| v.as_str())
            .unwrap_or("company");
        let fy_from = rec
            .get("from_year")
            .or_else(|| rec.get("fromYear"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fy_to = rec
            .get("to_year")
            .or_else(|| rec.get("toYear"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let submission_date = rec
            .get("submissionDate")
            .or_else(|| rec.get("submission_date"))
            .or_else(|| rec.get("broadcastDate"))
            .or_else(|| rec.get("date"))
            .or_else(|| rec.get("DT_TM"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let prefix = format!(
            "{}_{}_{}",
            sanitize_filename(company),
            sanitize_filename(fy_from),
            sanitize_filename(fy_to)
        )
        .trim_matches('_')
        .to_string();

        if let Some(obj) = rec.as_object() {
            for val in obj.values() {
                if let Some(raw_url) = val.as_str() {
                    let lo = raw_url.to_lowercase();
                    let is_pdf = lo.ends_with(".pdf") || lo.contains(".pdf?");
                    let is_xml =
                        lo.ends_with(".xml") || lo.contains(".xml?") || lo.ends_with(".xbrl");

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

                        let raw_leaf = full_url
                            .split('?')
                            .next()
                            .and_then(|u| u.split('/').next_back())
                            .unwrap_or("report");
                        let ext = match file_type {
                            FileType::Pdf => "pdf",
                            FileType::Xml => "xml",
                        };
                        let clean_leaf = sanitize_filename(
                            raw_leaf
                                .trim_end_matches(".pdf")
                                .trim_end_matches(".PDF")
                                .trim_end_matches(".xml")
                                .trim_end_matches(".XML")
                                .trim_end_matches(".xbrl")
                                .trim_end_matches(".XBRL"),
                        );
                        let file_name = build_safe_filename(&prefix, &clean_leaf, ext, 220);

                        let (upload_date, upload_time) = if !submission_date.trim().is_empty() {
                            parse_date_time_from_filename(submission_date)
                        } else {
                            parse_date_time_from_filename(&file_name)
                        };

                        files.push(PendingFile {
                            file_name,
                            download_url: full_url,
                            stock_market: Market::Nse,
                            file_type,
                            upload_date,
                            upload_time,
                            symbol: nse_symbol.clone(),
                            scrip_code: None,
                        });
                    }
                }
            }
        }
    }

    let mut seen_urls = HashSet::with_capacity(files.len());
    files.retain(|f| seen_urls.insert(f.download_url.clone()));
    Ok(files)
}
