// src/pipeline/scrapers.rs

pub mod engine;
pub mod scraper;
pub mod streaming;

pub use engine::ExchangeDownloader;
pub use scraper::{
    build_bse_client, build_nse_client, fetch_bse_records, fetch_nse_records, warmup_nse,
};
pub use streaming::{DownloadError, download_file_atomic};
