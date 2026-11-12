// src/pipeline.rs

pub mod geocoder;
pub mod ingestion;
pub mod scrapers;
pub mod xbrl;

pub use geocoder::GeocoderService;
pub use ingestion::IngestionManager;
pub use scrapers::ExchangeDownloader;
pub use xbrl::{FormDictionary, XbrlParser};
