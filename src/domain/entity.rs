// src/domain/entity.rs

use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct EntityProfile {
    pub corporate_id_number: String,
    pub company_name: String,
    pub address_registered_office: Option<String>,
    pub address_corporate_office: Option<String>,
    pub company_email: Option<String>,
    pub company_telephone: Option<String>,
    pub website: Option<String>,
    pub reporting_boundary: Option<String>,
    pub financial_year_start: Option<NaiveDate>,
    pub financial_year_end: Option<NaiveDate>,
    pub incorporation_date: Option<NaiveDate>,
    pub paid_up_shares_value: Option<f64>,
    pub contact_person_name: Option<String>,
    pub contact_person_phone: Option<String>,
    pub contact_person_email: Option<String>,
    pub isin: Option<String>,
    pub nse_symbol: Option<String>,
    pub bse_scrip_code: Option<String>,
}

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
