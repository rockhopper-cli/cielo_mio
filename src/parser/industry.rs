// src/parser/industry.rs

use sqlx::{PgPool, Row};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IndustryRecord {
    pub nic_code: String,
    pub division: String,
    pub sector: String,
}

#[derive(Debug, Clone, Default)]
pub struct IndustryRegistry {
    // Exact 5-digit match
    by_nic: HashMap<String, IndustryRecord>,
    // Fallback 2-digit division match
    by_division: HashMap<String, IndustryRecord>,
}

impl IndustryRegistry {
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT nic_code, division_code, division_name, esg_sector FROM industry_master",
        )
        .fetch_all(pool)
        .await?;

        let mut by_nic = HashMap::with_capacity(rows.len());
        let mut by_division = HashMap::new();

        for r in rows {
            let nic: String = r.get("nic_code");
            let div_code: String = r.get("division_code");
            let div_name: String = r.get("division_name");
            let sector: String = r.get("esg_sector");

            let record = IndustryRecord {
                nic_code: nic.clone(),
                division: div_name,
                sector,
            };

            by_division
                .entry(div_code)
                .or_insert_with(|| record.clone());
            by_nic.insert(nic, record);
        }

        Ok(Self {
            by_nic,
            by_division,
        })
    }

    pub fn classify_cin(&self, cin: &str) -> IndustryRecord {
        let clean = cin.trim().to_uppercase();
        if clean.len() >= 6 {
            let nic = &clean[1..6];
            let division = &clean[1..3];

            // 1. Try exact 5-digit NIC lookup
            if let Some(record) = self.by_nic.get(nic) {
                return record.clone();
            }

            // 2. Fallback to 2-digit division lookup
            if let Some(record) = self.by_division.get(division) {
                return IndustryRecord {
                    nic_code: nic.to_string(),
                    division: record.division.clone(),
                    sector: record.sector.clone(),
                };
            }
        }

        IndustryRecord {
            nic_code: clean.chars().skip(1).take(5).collect(),
            division: "Unclassified Division".to_string(),
            sector: "Diversified / Other".to_string(),
        }
    }
}
