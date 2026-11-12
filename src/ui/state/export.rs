// src/app/state/export.rs

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct ExportState {
    pub export_scope_entity: String,
    pub export_scope_sector: String,
    pub export_scope_grouping: String,
    pub export_format_xlsx: bool,
    pub include_unmapped: bool,
    pub is_exporting: Arc<AtomicBool>,
    pub is_dumping_db: Arc<AtomicBool>,
    pub feedback_message: Option<(String, bool)>,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            export_scope_entity: String::new(),
            export_scope_sector: String::new(),
            export_scope_grouping: String::new(),
            export_format_xlsx: false,
            include_unmapped: false,
            is_exporting: Arc::new(AtomicBool::new(false)),
            is_dumping_db: Arc::new(AtomicBool::new(false)),
            feedback_message: None,
        }
    }
}
