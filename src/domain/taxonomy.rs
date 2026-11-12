// src/domain/taxonomy.rs

use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IndustryRecord {
    pub nic_code: String,
    pub division: String,
    pub sector: String,
}

#[derive(Debug, Clone, Default)]
pub struct IndustryRegistry {
    by_nic: HashMap<String, IndustryRecord>,
    by_division: HashMap<String, IndustryRecord>,
}

impl IndustryRegistry {
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows = crate::storage::repository::taxonomy::fetch_industry_master(pool).await?;

        let mut by_nic = HashMap::with_capacity(rows.len());
        let mut by_division = HashMap::new();

        for (nic, div_code, div_name, sector) in rows {
            let record = IndustryRecord {
                nic_code: nic.clone(),
                division: div_name,
                sector,
            };

            if nic == div_code || nic.len() == 2 {
                by_division.insert(div_code, record.clone());
            } else {
                by_division
                    .entry(div_code)
                    .or_insert_with(|| record.clone());
            }
            by_nic.insert(nic, record);
        }

        Ok(Self {
            by_nic,
            by_division,
        })
    }

    /// Classifies an Indian CIN string (e.g. L01111MH2000PLC123456), with statutory company-name fallback
    pub fn classify_cin_or_name(&self, cin: &str, company_name: &str) -> IndustryRecord {
        let clean = cin.trim().to_uppercase();
        let is_valid_mca_cin = clean.is_ascii()
            && clean.len() == 21
            && matches!(clean.as_bytes()[0], b'L' | b'U')
            && clean[1..6].bytes().all(|b| b.is_ascii_digit());

        if is_valid_mca_cin {
            let nic = &clean[1..6];
            let division = &clean[1..3];

            if let Some(record) = self.by_nic.get(nic) {
                return record.clone();
            }
            if let Some(record) = self.by_division.get(division) {
                return IndustryRecord {
                    nic_code: nic.to_string(),
                    division: record.division.clone(),
                    sector: record.sector.clone(),
                };
            }
        }

        let upper_name = company_name.trim().to_uppercase();
        if upper_name.contains("BANK")
            || upper_name.contains("FINANCE")
            || upper_name.contains("FINSERV")
            || upper_name.contains("INSURANCE")
            || upper_name.contains("ASSURANCE")
        {
            return IndustryRecord {
                nic_code: String::new(),
                division: "Banking, Financial Services & Insurance".to_string(),
                sector: "Banking & Financial Services".to_string(),
            };
        }

        IndustryRecord {
            nic_code: String::new(),
            division: "Pending Exchange Mapping".to_string(),
            sector: "Unclassified".to_string(),
        }
    }

    pub fn classify_cin(&self, cin: &str) -> IndustryRecord {
        self.classify_cin_or_name(cin, "")
    }
}

#[derive(Debug, Clone)]
pub struct FormRule {
    pub indicator_capmkt: String,
    pub context_reference: String,
    pub standard_indicator_capmkt: String,
    pub standard_context_reference: String,
    pub section: String,
    pub principle_number: i16,
    pub sub_section: i16,
    pub indicator_type: String,
    pub friendly_grouping: Option<String>,
}
