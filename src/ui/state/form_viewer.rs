// src/ui/state/form_viewer.rs

use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CompanyFilingSummary {
    pub entity_id: Uuid,
    pub corporate_id_number: String,
    pub company_name: String,
    pub address_registered_office: Option<String>,
    pub address_corporate_office: Option<String>,
    pub company_email: Option<String>,
    pub company_telephone: Option<String>,
    pub website: Option<String>,
    pub paid_up_shares_value: Option<Decimal>,
    pub fy_start: Option<NaiveDate>,
    pub fy_end: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FormMasterItem {
    pub question_key: String,
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub question_in_form: Option<i16>,
    pub question_number: i16,
    pub followup_question_first: String,
    pub followup_question_second: String,
    pub question_order: i16,
    pub question_type: String,
    pub response_datatype: String,
    pub contextref_increment: bool,
    pub total_field: bool,
    pub friendly_grouping: Option<String>,
    pub field_flag: i16,
    pub anchor_reference: i16,
    pub append_to_field: Option<String>,
    pub section_text: String,
    pub principle_text: String,
    pub sub_section_text: String,
    pub indicator_text: String,
    pub question_text: String,
    pub standard_indicator_capmkt: String,
    pub standard_context_reference: String,
    pub input_text: Option<String>,
    pub visibility_dependence: String,
    pub visibility_trigger: Option<String>,
}

#[derive(Debug)]
pub enum FormViewerEvent {
    FilingsLoaded(Vec<CompanyFilingSummary>),
    TemplateLoaded(Vec<FormMasterItem>),
    FactsLoaded {
        entity_id: Uuid,
        facts: Vec<FormFactRow>,
    },
    Error(String),
}

pub struct FormViewerState {
    pub available_filings: Vec<CompanyFilingSummary>,
    pub unique_companies: Vec<(String, String)>,
    pub selected_cin: String,
    pub selected_entity_id: Option<Uuid>,
    pub filings_loaded: bool,
    pub company_search: String,

    pub selected_section: String,
    pub selected_sub_section: i16,
    pub selected_principle: i16,
    pub selected_indicator_type: Option<String>,

    pub form_template: Vec<FormMasterItem>,
    pub template_loaded: bool,

    pub current_facts: Vec<FormFactRow>,
    pub cached_entity_id: Option<Uuid>,
    pub is_loading_facts: bool,

    pub search_query: String,
}

impl Default for FormViewerState {
    fn default() -> Self {
        Self {
            available_filings: Vec::new(),
            unique_companies: Vec::new(),
            selected_cin: String::new(),
            selected_entity_id: None,
            filings_loaded: false,
            company_search: String::new(),
            selected_section: "A".to_string(),
            selected_sub_section: 0,
            selected_principle: 0,
            selected_indicator_type: None,
            form_template: Vec::new(),
            template_loaded: false,
            current_facts: Vec::new(),
            cached_entity_id: None,
            is_loading_facts: false,
            search_query: String::new(),
        }
    }
}
