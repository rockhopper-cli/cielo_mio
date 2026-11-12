// src/ai/tools.rs

use rust_decimal::prelude::ToPrimitive;
use sqlx::{PgPool, Row};
use std::fmt::Write;
use uuid::Uuid;

pub struct McpTools {
    pool: PgPool,
}

impl McpTools {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn resolve_entity_id(&self, query: &str) -> Result<Option<Uuid>, sqlx::Error> {
        let clean = query.trim();
        if clean.is_empty() {
            return Ok(None);
        }
        let pattern = format!("%{}%", clean);
        let row = sqlx::query(
            r#"
            SELECT entity_id
            FROM entity_list
            WHERE corporate_id_number ILIKE $1 OR company_name ILIKE $1
            ORDER BY (corporate_id_number ILIKE $2) DESC, financial_year_start DESC NULLS LAST
            LIMIT 1;
            "#,
        )
        .bind(&pattern)
        .bind(clean)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("entity_id")))
    }

    /// Fetches company profile and basic metadata by company name or CIN
    pub async fn get_company_profile(&self, query: &str) -> Result<String, sqlx::Error> {
        let clean = query.trim();
        if clean.is_empty() {
            return Ok("No target entity specified.".to_string());
        }

        let sql = r#"
            SELECT 
                company_name, corporate_id_number, address_registered_office,
                company_email, financial_year_start, financial_year_end,
                paid_up_shares_value, stock_market,
                COALESCE(industry_sector, 'Unclassified') as industry_sector,
                COALESCE(industry_division, 'General') as industry_division
            FROM entity_list
            WHERE corporate_id_number ILIKE $1 
               OR company_name ILIKE $1
            ORDER BY (corporate_id_number ILIKE $2) DESC, financial_year_start DESC NULLS LAST
            LIMIT 1;
        "#;

        let pattern = format!("%{}%", clean);
        let row = sqlx::query(sql)
            .bind(&pattern)
            .bind(clean)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => {
                let name: String = r.get("company_name");
                let cin: String = r.get("corporate_id_number");
                let sector: String = r.get("industry_sector");
                let div: String = r.get("industry_division");
                let addr: Option<String> = r.get("address_registered_office");
                let email: Option<String> = r.get("company_email");
                let paid_up: Option<rust_decimal::Decimal> = r.get("paid_up_shares_value");
                let market: String = r.get("stock_market");
                let fy_start: Option<chrono::NaiveDate> = r.get("financial_year_start");
                let fy_end: Option<chrono::NaiveDate> = r.get("financial_year_end");

                let mut out = String::new();
                writeln!(out, "Company Profile:").unwrap();
                writeln!(out, "  • Name: {}", name).unwrap();
                writeln!(out, "  • CIN: {}", cin).unwrap();
                writeln!(out, "  • Exchange: {}", market).unwrap();
                writeln!(out, "  • Sector: {} (Division: {})", sector, div).unwrap();
                if let (Some(s), Some(e)) = (fy_start, fy_end) {
                    writeln!(out, "  • Financial Year: {} to {}", s, e).unwrap();
                }
                writeln!(
                    out,
                    "  • Registered Address: {}",
                    addr.unwrap_or_else(|| "N/A".to_string())
                )
                .unwrap();
                writeln!(
                    out,
                    "  • Contact Email: {}",
                    email.unwrap_or_else(|| "N/A".to_string())
                )
                .unwrap();
                if let Some(val) = paid_up {
                    let cr_val = val.to_f64().unwrap_or(0.0) / 10_000_000.0;
                    writeln!(out, "  • Paid-up Capital: ₹ {:.2} Cr", cr_val).unwrap();
                }
                Ok(out)
            }
            None => Ok(format!(
                "No company profile found in repository for '{}'.",
                clean
            )),
        }
    }

    /// Queries standardized disclosures for the resolved entity matching an ESG keyword
    pub async fn query_company_metrics(
        &self,
        entity_query: &str,
        keyword: &str,
    ) -> Result<String, sqlx::Error> {
        let clean_ent = entity_query.trim();
        let clean_kw = keyword.trim();
        if clean_ent.is_empty() || clean_kw.is_empty() {
            return Ok(String::new());
        }

        let Some(entity_id) = self.resolve_entity_id(clean_ent).await? else {
            return Ok(String::new());
        };

        let sql = r#"
            SELECT 
                f.section,
                f.principle_number,
                COALESCE(f.friendly_grouping, 'general') as friendly_grouping,
                f.standard_indicator_capmkt,
                COALESCE(f.standard_context_reference, f.context_reference, '') as context_ref,
                f.raw_value,
                f.standard_numeric_value,
                f.standard_unit_reference
            FROM brsr_facts f
            WHERE f.client_uuid = $1
              AND (f.friendly_grouping ILIKE $2
                   OR f.standard_indicator_capmkt ILIKE $2 
                   OR f.indicator_capmkt ILIKE $2
                   OR f.raw_value ILIKE $2)
            ORDER BY
                CASE WHEN f.standard_context_reference ~* '(_p?py|previous|prior)' THEN 1 ELSE 0 END ASC,
                f.section, f.principle_number, f.friendly_grouping, f.standard_indicator_capmkt
            LIMIT 40;
        "#;

        let kw_pattern = format!("%{}%", clean_kw);

        let rows = sqlx::query(sql)
            .bind(entity_id)
            .bind(kw_pattern)
            .fetch_all(&self.pool)
            .await?;

        if rows.is_empty() {
            return Ok(String::new());
        }

        let mut out = String::new();
        writeln!(
            out,
            "Repository Disclosures for keyword/table '{}':",
            clean_kw
        )
        .unwrap();
        for r in rows {
            let sec: String = r.get("section");
            let p: i16 = r.get("principle_number");
            let group: String = r.get("friendly_grouping");
            let tag: String = r.get("standard_indicator_capmkt");
            let ctx: String = r.get("context_ref");
            let raw: Option<String> = r.get("raw_value");
            let std_num: Option<rust_decimal::Decimal> = r.get("standard_numeric_value");
            let std_unit: Option<String> = r.get("standard_unit_reference");

            write!(
                out,
                "  • [Sec {}-P{} | {}] {} ({}) = ",
                sec, p, group, tag, ctx
            )
            .unwrap();
            if let (Some(num), Some(unit)) = (std_num, std_unit) {
                writeln!(out, "{} {}", num, unit).unwrap();
            } else {
                writeln!(out, "{}", raw.unwrap_or_else(|| "N/A".to_string())).unwrap();
            }
        }

        Ok(out)
    }
}
