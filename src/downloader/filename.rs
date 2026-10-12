// src/downloader/filename.rs

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static RE_14: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{14})").unwrap());
static RE_8_6: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{8})[_-]?(\d{6})").unwrap());
static RE_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{8})").unwrap());
static RE_6YM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(20\d{4})").unwrap());
static RE_SANITIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[\\/*?:"<>|]+"#).unwrap());

/// Parses upload_date and upload_time from a filename or exchange timestamp string.
pub fn parse_date_time_from_filename(filename: &str) -> (NaiveDate, NaiveTime) {
    let t = filename.trim();
    let default_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let today = Local::now().date_naive();

    // 1. FAST PATH: Check explicit Exchange API datetime strings (BSE DT_TM / submission dates)
    const DATE_FORMATS: &[&str] = &[
        "%d/%m/%Y %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%d-%m-%Y %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%d/%m/%Y",
        "%Y/%m/%d",
        "%Y-%m-%d",
        "%d-%m-%Y",
    ];

    for fmt in DATE_FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(t, fmt) {
            return (dt.date(), dt.time());
        }
        if let Ok(d) = NaiveDate::parse_from_str(t, fmt) {
            return (d, default_time);
        }
    }

    // 2. FILENAME PATH: Isolate filename stem to strip extensions (.xml, .pdf) & folder paths
    let base_name = Path::new(t)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(t);

    // Pattern 1: 14-digit contiguous timestamp (DDMMYYYYHHMMSS)
    if let Some(caps) = RE_14.captures(base_name) {
        if let Some(matched) = caps.get(1) {
            let s = matched.as_str();
            let dd: u32 = s[0..2].parse().unwrap_or(1).max(1);
            let mm: u32 = s[2..4].parse().unwrap_or(1);
            let yyyy: i32 = s[4..8].parse().unwrap_or(1970);
            let hh: u32 = s[8..10].parse().unwrap_or(0);
            let mi: u32 = s[10..12].parse().unwrap_or(0);
            let ss: u32 = s[12..14].parse().unwrap_or(0);

            if let (Some(d), Some(tm)) = (
                NaiveDate::from_ymd_opt(yyyy, mm, dd),
                NaiveTime::from_hms_opt(hh, mi, ss),
            ) {
                return (d, tm);
            }
        }
    }

    // Pattern 2: 8-digit date + 6-digit time (YYYYMMDD_HHMMSS or DDMMYYYY_HHMMSS)
    if let Some(caps) = RE_8_6.captures(base_name) {
        let date_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let time_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let hh: u32 = time_part[0..2].parse().unwrap_or(0);
        let mi: u32 = time_part[2..4].parse().unwrap_or(0);
        let ss: u32 = time_part[4..6].parse().unwrap_or(0);
        let parsed_time = NaiveTime::from_hms_opt(hh, mi, ss).unwrap_or(default_time);

        // Try DDMMYYYY
        let d1: u32 = date_part[0..2].parse().unwrap_or(1).max(1);
        let m1: u32 = date_part[2..4].parse().unwrap_or(1);
        let y1: i32 = date_part[4..8].parse().unwrap_or(1970);
        if let Some(d) = NaiveDate::from_ymd_opt(y1, m1, d1) {
            return (d, parsed_time);
        }

        // Try YYYYMMDD
        let y2: i32 = date_part[0..4].parse().unwrap_or(1970);
        let m2: u32 = date_part[4..6].parse().unwrap_or(1);
        let d2: u32 = date_part[6..8].parse().unwrap_or(1).max(1);
        if let Some(d) = NaiveDate::from_ymd_opt(y2, m2, d2) {
            return (d, parsed_time);
        }
    }

    // Pattern 3: 8-digit date only (YYYYMMDD or DDMMYYYY)
    if let Some(caps) = RE_8.captures(base_name) {
        let s = caps.get(1).map(|m| m.as_str()).unwrap_or("");

        let d1: u32 = s[0..2].parse().unwrap_or(1).max(1);
        let m1: u32 = s[2..4].parse().unwrap_or(1);
        let y1: i32 = s[4..8].parse().unwrap_or(1970);
        if let Some(d) = NaiveDate::from_ymd_opt(y1, m1, d1) {
            return (d, default_time);
        }

        let y2: i32 = s[0..4].parse().unwrap_or(1970);
        let m2: u32 = s[4..6].parse().unwrap_or(1);
        let d2: u32 = s[6..8].parse().unwrap_or(1).max(1);
        if let Some(d) = NaiveDate::from_ymd_opt(y2, m2, d2) {
            return (d, default_time);
        }
    }

    // Pattern 4: 6-digit Year-Month (e.g. 202308 -> 2023-08-01)
    if let Some(caps) = RE_6YM.captures(base_name) {
        let s = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let y: i32 = s[0..4].parse().unwrap_or(1970);
        let m: u32 = s[4..6].parse().unwrap_or(1);
        if let Some(d) = NaiveDate::from_ymd_opt(y, m, 1) {
            return (d, default_time);
        }
    }

    (today, default_time)
}

/// Sanitizes company or report names into safe filesystem strings
pub fn sanitize_filename(s: &str) -> String {
    RE_SANITIZE.replace_all(s, "").trim().replace(' ', "_")
}
