use chrono::{NaiveDate, NaiveTime};
use std::path::PathBuf;

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

/// User-controlled download settings
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub market: Market,
    pub output_dir: PathBuf,
    pub download_pdf: bool,
    pub download_xml: bool,
    pub max_files: Option<usize>, // None = download all, Some(n) = test mode limit
}

/// Metadata extracted prior to downloading
#[derive(Debug, Clone)]
pub struct PendingFile {
    pub file_name: String,
    pub download_url: String,
    pub stock_market: Market,
    pub file_type: FileType,
    pub upload_date: NaiveDate,
    pub upload_time: NaiveTime,
}

/// Status returned when querying `file_list`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSyncStatus {
    /// Already mapped to database tables -> SKIP
    AlreadyMapped,
    /// Previously logged with a failure -> RETRY
    FailedPreviously(Option<String>),
    /// Never encountered -> DOWNLOAD
    NewFile,
}

/// Events emitted to decouple engine from UI logging
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
