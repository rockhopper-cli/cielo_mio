// src/domain/facts.rs

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct RawFact {
    pub client_id: String,
    pub in_capmkt: String,
    pub context_ref: String,
    pub unit_ref: Option<String>,
    pub decimals: Option<i16>,
    pub raw_value: String,
    pub standard_unit_ref: Option<String>,
    pub numeric_value: Option<Decimal>,
    pub standard_value: Option<Decimal>,
    pub standard_value_str: Option<String>,
    pub converted_unit: Option<String>,
    pub datatype: &'static str,
}

#[derive(Debug, Clone)]
pub struct ParsedFiling {
    pub client_id: String,
    pub detected_isin: Option<String>,
    pub detected_symbol: Option<String>,
    pub detected_scrip_code: Option<String>,
    pub facts: Vec<RawFact>,
}

#[derive(Debug, Clone)]
pub struct EnrichedFact {
    pub client_id: String,
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub indicator_capmkt: String,
    pub standard_indicator_capmkt: String,
    pub context_reference: String,
    pub standard_context_reference: String,
    pub friendly_grouping: Option<String>,
    pub response_datatype: &'static str,
    pub raw_value: String,
    pub numeric_value: Option<Decimal>,
    pub unit_reference: Option<String>,
    pub standard_unit_reference: Option<String>,
    pub decimals: Option<i16>,
    pub standard_raw_value: Option<String>,
    pub standard_numeric_value: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct FormFactRow {
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub standard_indicator: String,
    pub context_ref: String,
    pub friendly_grouping: Option<String>,
    pub raw_value: String,
    pub standard_numeric_value: Option<Decimal>,
    pub standard_unit: Option<String>,
}
