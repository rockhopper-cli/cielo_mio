// src/pipeline/xbrl.rs

pub mod parser;
pub mod transform;

pub use parser::{XbrlParser, sanitize_xml_string};
pub use transform::FormDictionary;
