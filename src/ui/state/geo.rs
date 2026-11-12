// src/ui/state/geo.rs

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use uuid::Uuid;
use walkers::{HttpTiles, MapMemory};

#[derive(Debug, Clone)]
pub struct GeoEntityMarker {
    pub entity_id: Uuid,
    pub company_name: String,
    pub corporate_id_number: String,
    pub industry_sector: String,
    pub latitude: f64,
    pub longitude: f64,
    pub state_name: String,
    pub city_name: String,
    pub scope_1_tco2e: Option<f64>,
    pub water_withdrawal_kl: Option<f64>,
}

pub struct GeoReportsState {
    pub markers: Vec<GeoEntityMarker>,
    pub is_loading: bool,
    pub is_geocoding_running: Arc<AtomicBool>,
    pub selected_sector: String,
    pub selected_state: String,
    pub search_query: String,
    // Walkers persistent map state
    pub tiles: Option<HttpTiles>,
    pub map_memory: MapMemory,
    pub selected_entity_id: Option<Uuid>,
}

impl Default for GeoReportsState {
    fn default() -> Self {
        Self {
            markers: Vec::new(),
            is_loading: false,
            is_geocoding_running: Arc::new(AtomicBool::new(false)),
            selected_sector: "All Sectors".to_string(),
            selected_state: "All States".to_string(),
            search_query: String::new(),
            tiles: None,
            map_memory: MapMemory::default(),
            selected_entity_id: None,
        }
    }
}
