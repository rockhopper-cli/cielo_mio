// src/domain/download.rs

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static RE_14: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{14})").unwrap());
static RE_8_6: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{8})[_-]?(\d{6})").unwrap());
static RE_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{8})").unwrap());
static RE_6YM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(20\d{4})").unwrap());
static RE_SANITIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[\\/*?:"<>|]+"#).unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Market {
    Bse,
    Nse,
}

impl Market {
    pub fn as_str(&self) -> &'static str {
        match self {
            Market::Bse => "BSE",
            Market::Nse => "NSE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Pdf,
    Xml,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Pdf => "PDF",
            FileType::Xml => "XML",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub market: Market,
    pub output_dir: PathBuf,
    pub download_pdf: bool,
    pub download_xml: bool,
    pub max_files: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PendingFile {
    pub file_name: String,
    pub download_url: String,
    pub stock_market: Market,
    pub file_type: FileType,
    pub upload_date: NaiveDate,
    pub upload_time: NaiveTime,
    pub symbol: Option<String>,
    pub scrip_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSyncStatus {
    AlreadyMapped,
    FailedPreviously(Option<String>),
    NewFile,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Log(String),
    FileSkipped {
        file_name: String,
        reason: String,
    },
    FileDownloaded {
        file_name: String,
        path: PathBuf,
    },
    FileFailed {
        file_name: String,
        error: String,
    },
    Done {
        total_downloaded: usize,
        total_skipped: usize,
    },
}

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

fn try_parse_8_digit_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    // Check YYYYMMDD first when starting with 19xx or 20xx
    let y_first: i32 = s[0..4].parse().ok()?;
    if (1990..=2100).contains(&y_first) {
        let m: u32 = s[4..6].parse().ok()?;
        let d: u32 = s[6..8].parse().ok()?;
        if let Some(date) = NaiveDate::from_ymd_opt(y_first, m, d) {
            return Some(date);
        }
    }

    // Otherwise check DDMMYYYY with realistic filing year bounds
    let y_last: i32 = s[4..8].parse().ok()?;
    if (1990..=2100).contains(&y_last) {
        let d: u32 = s[0..2].parse().ok()?;
        let m: u32 = s[2..4].parse().ok()?;
        if let Some(date) = NaiveDate::from_ymd_opt(y_last, m, d) {
            return Some(date);
        }
    }

    None
}

/// Parses upload_date and upload_time from a filename or exchange timestamp string
pub fn parse_date_time_from_filename(filename: &str) -> (NaiveDate, NaiveTime) {
    let t = filename.trim();
    let default_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let today = Local::now().date_naive();

    const DATE_FORMATS: &[&str] = &[
        "%d/%m/%Y %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%d-%m-%Y %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%d-%b-%Y %H:%M:%S",
        "%d %b %Y %H:%M:%S",
        "%d/%m/%Y",
        "%Y/%m/%d",
        "%Y-%m-%d",
        "%d-%m-%Y",
        "%d-%b-%Y",
    ];

    for fmt in DATE_FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(t, fmt) {
            return (dt.date(), dt.time());
        }
        if let Ok(d) = NaiveDate::parse_from_str(t, fmt) {
            return (d, default_time);
        }
    }

    let base_name = Path::new(t)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(t);

    if let Some(caps) = RE_14.captures(base_name)
        && let Some(matched) = caps.get(1)
    {
        let s = matched.as_str();
        let hh: u32 = s[8..10].parse().unwrap_or(0);
        let mi: u32 = s[10..12].parse().unwrap_or(0);
        let ss: u32 = s[12..14].parse().unwrap_or(0);

        if let (Some(d), Some(tm)) = (
            try_parse_8_digit_date(&s[0..8]),
            NaiveTime::from_hms_opt(hh, mi, ss),
        ) {
            return (d, tm);
        }
    }

    if let Some(caps) = RE_8_6.captures(base_name) {
        let date_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let time_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let hh: u32 = time_part[0..2].parse().unwrap_or(0);
        let mi: u32 = time_part[2..4].parse().unwrap_or(0);
        let ss: u32 = time_part[4..6].parse().unwrap_or(0);
        let parsed_time = NaiveTime::from_hms_opt(hh, mi, ss).unwrap_or(default_time);

        if let Some(d) = try_parse_8_digit_date(date_part) {
            return (d, parsed_time);
        }
    }

    if let Some(caps) = RE_8.captures(base_name)
        && let Some(m) = caps.get(1)
        && let Some(d) = try_parse_8_digit_date(m.as_str())
    {
        return (d, default_time);
    }

    if let Some(caps) = RE_6YM.captures(base_name) {
        let s = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let y: i32 = s[0..4].parse().unwrap_or(1970);
        let m: u32 = s[4..6].parse().unwrap_or(1);
        if (1990..=2100).contains(&y)
            && let Some(d) = NaiveDate::from_ymd_opt(y, m, 1)
        {
            return (d, default_time);
        }
    }

    (today, default_time)
}

/// Sanitizes company or report names into safe filesystem strings
pub fn sanitize_filename(s: &str) -> String {
    let cleaned = RE_SANITIZE
        .replace_all(s, "")
        .trim()
        .replace([' ', '\t', '\r', '\n'], "_");
    // Collapse repeated underscores
    let mut out = String::with_capacity(cleaned.len());
    let mut prev_us = false;
    for ch in cleaned.chars() {
        if ch == '_' {
            if !prev_us {
                out.push('_');
            }
            prev_us = true;
        } else {
            out.push(ch);
            prev_us = false;
        }
    }
    out.trim_matches('_').to_string()
}
