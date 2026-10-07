// src/parser/transform.rs

use crate::parser::units::parse_number;
use crate::parser::xml::RawFact;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

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
    pub numeric_value: Option<f64>,
    pub unit_reference: Option<String>,
    pub standard_unit_reference: Option<String>,
    pub decimals: Option<i16>,
    pub standard_raw_value: Option<String>,
    pub standard_numeric_value: Option<f64>,
}

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
}

pub struct FormDictionary {
    rules: HashMap<(String, String), FormRule>,
    increment_rules: Vec<FormRule>,
}

impl FormDictionary {
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                indicator_capmkt,
                context_reference,
                standard_indicator_capmkt,
                standard_context_reference,
                section,
                principle_number,
                sub_section,
                indicator_type,
                friendly_grouping,
                contextref_increment
            FROM form_master
            WHERE indicator_capmkt IS NOT NULL 
              AND context_reference IS NOT NULL 
              AND indicator_capmkt != '' 
              AND context_reference != '';
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut rules = HashMap::with_capacity(rows.len());
        let mut increment_rules = Vec::new();

        for row in rows {
            let indicator: String = row.get("indicator_capmkt");
            let context: String = row.get("context_reference");
            let std_indicator: Option<String> = row.get("standard_indicator_capmkt");
            let std_context: Option<String> = row.get("standard_context_reference");
            let section: String = row.get("section");
            let principle: i16 = row.get("principle_number");
            let sub_sec: i16 = row.get("sub_section");
            let ind_type: String = row.get("indicator_type");
            let friendly: Option<String> = row.get("friendly_grouping");
            let inc: bool = row.get("contextref_increment");

            let rule = FormRule {
                indicator_capmkt: indicator.clone(),
                context_reference: context.clone(),
                standard_indicator_capmkt: std_indicator.unwrap_or_else(|| indicator.clone()),
                standard_context_reference: std_context.unwrap_or_else(|| context.clone()),
                section,
                principle_number: principle,
                sub_section: sub_sec,
                indicator_type: ind_type,
                friendly_grouping: friendly,
            };

            if inc {
                increment_rules.push(rule.clone());
            }

            rules.insert((indicator, context), rule);
        }

        Ok(Self {
            rules,
            increment_rules,
        })
    }

    pub fn enrich_facts(
        &self,
        filing_cin: &str,
        facts: &[RawFact],
    ) -> (Vec<EnrichedFact>, EntityProfile) {
        let mut active_rules = self.rules.clone();

        // Dynamic context expansion matching python logic
        for inc_rule in &self.increment_rules {
            let prefix = inc_rule
                .context_reference
                .trim_end_matches(|c: char| c.is_ascii_digit());
            let std_prefix = inc_rule
                .standard_context_reference
                .trim_end_matches(|c: char| c.is_ascii_digit());

            for fact in facts {
                if fact.in_capmkt == inc_rule.indicator_capmkt
                    && fact.context_ref.starts_with(prefix)
                {
                    let key = (fact.in_capmkt.clone(), fact.context_ref.clone());
                    if !active_rules.contains_key(&key) {
                        let suffix = &fact.context_ref[prefix.len()..];
                        let mut cloned = inc_rule.clone();
                        cloned.context_reference = fact.context_ref.clone();
                        cloned.standard_context_reference = format!("{}{}", std_prefix, suffix);
                        active_rules.insert(key, cloned);
                    }
                }
            }
        }

        let mut enriched = Vec::with_capacity(facts.len());
        let mut entity_map: HashMap<String, String> = HashMap::new();
        let mut seen_facts: HashSet<(String, String, Option<String>, String)> = HashSet::new();

        for fact in facts {
            let clean_val = fact.raw_value.trim();
            if clean_val.is_empty() {
                continue;
            }

            let key = (fact.in_capmkt.clone(), fact.context_ref.clone());

            let (section, principle, sub_sec, ind_type, std_tag, std_ctx, friendly) =
                match active_rules.get(&key) {
                    Some(r) => (
                        r.section.clone(),
                        r.principle_number,
                        r.sub_section,
                        r.indicator_type.clone(),
                        r.standard_indicator_capmkt.clone(),
                        r.standard_context_reference.clone(),
                        r.friendly_grouping.clone(),
                    ),
                    None => (
                        "A".to_string(),
                        0i16,
                        0i16,
                        "General".to_string(),
                        fact.in_capmkt.clone(),
                        fact.context_ref.clone(),
                        None,
                    ),
                };

            // Capture facts for entity profile
            let is_entity_group = friendly
                .as_ref()
                .map(|fg| fg.eq_ignore_ascii_case("entity_list"))
                .unwrap_or(false);

            if is_entity_group || is_profile_tag(&std_tag) || is_profile_tag(&fact.in_capmkt) {
                entity_map.insert(std_tag.clone(), clean_val.to_string());
                entity_map.insert(fact.in_capmkt.clone(), clean_val.to_string());
            }

            // Deduplicate matching records
            let dedupe_key = (
                std_tag.clone(),
                std_ctx.clone(),
                fact.unit_ref.clone(),
                clean_val.to_string(),
            );
            if !seen_facts.insert(dedupe_key) {
                continue;
            }

            // Resolve the final standardized unit:
            // 1. Use the converted canonical unit (e.g. "GJ", "t", "KL")
            // 2. Fallback to the resolved XML unit measure if no conversion was needed
            // 3. Fallback to the raw unitRef attribute
            let final_std_unit = fact
                .converted_unit
                .clone()
                .or_else(|| fact.standard_unit_ref.clone())
                .or_else(|| fact.unit_ref.clone());

            enriched.push(EnrichedFact {
                client_id: if !fact.client_id.is_empty() {
                    fact.client_id.clone()
                } else {
                    filing_cin.to_string()
                },
                section,
                principle_number: principle,
                sub_section: sub_sec,
                indicator_type: ind_type,
                indicator_capmkt: fact.in_capmkt.clone(),
                standard_indicator_capmkt: std_tag,
                context_reference: fact.context_ref.clone(),
                standard_context_reference: std_ctx,
                friendly_grouping: friendly,
                response_datatype: fact.datatype,
                raw_value: clean_val.to_string(),
                numeric_value: parse_number(clean_val),
                unit_reference: fact.unit_ref.clone(),
                standard_unit_reference: final_std_unit,
                decimals: fact.decimals,
                standard_raw_value: Some(clean_val.to_string()),
                standard_numeric_value: fact.standard_value,
            });
        }

        let resolved_cin = entity_map
            .remove("CorporateIdentityNumber")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| filing_cin.to_string());

        let profile = EntityProfile {
            corporate_id_number: resolved_cin,
            company_name: entity_map
                .remove("NameOfTheCompany")
                .or_else(|| entity_map.remove("CompanyName"))
                .unwrap_or_else(|| "Unknown Company".to_string()),
            address_registered_office: entity_map
                .remove("AddressOfRegisteredOfficeOfCompany")
                .or_else(|| entity_map.remove("RegisteredOfficeAddress")),
            address_corporate_office: entity_map
                .remove("AddressOfCorporateOfficeOfCompany")
                .or_else(|| entity_map.remove("CorporateOfficeAddress")),
            company_email: entity_map
                .remove("EMailOfTheCompany")
                .or_else(|| entity_map.remove("EmailOfTheCompany")),
            company_telephone: entity_map
                .remove("TelephoneOfCompany")
                .or_else(|| entity_map.remove("TelephoneNumber")),
            website: entity_map
                .remove("WebsiteOfCompany")
                .or_else(|| entity_map.remove("Website")),
            reporting_boundary: entity_map.remove("ReportingBoundary"),
            financial_year_start: entity_map
                .get("DateOfStartOfFinancialYear")
                .or_else(|| entity_map.get("FinancialYearStartDate"))
                .and_then(|s| parse_date(s)),
            financial_year_end: entity_map
                .get("DateOfEndOfFinancialYear")
                .or_else(|| entity_map.get("FinancialYearEndDate"))
                .and_then(|s| parse_date(s)),
            incorporation_date: entity_map
                .get("DateOfIncorporation")
                .or_else(|| entity_map.get("YearOfIncorporation"))
                .and_then(|s| parse_date(s)),
            paid_up_shares_value: entity_map
                .get("PaidUpValueOfShares")
                .or_else(|| entity_map.get("ValueOfSharesPaidUp"))
                .and_then(|s| parse_number(s)),
            contact_person_name: entity_map.remove("NameOfContactPerson"),
            contact_person_phone: entity_map
                .remove("ContactNumberOfContactPerson")
                .or_else(|| entity_map.remove("TelephoneOfContactPerson")),
            contact_person_email: entity_map
                .remove("EMailOfContactPerson")
                .or_else(|| entity_map.remove("EmailOfContactPerson")),
        };

        (enriched, profile)
    }
}

fn is_profile_tag(tag: &str) -> bool {
    matches!(
        tag,
        "CorporateIdentityNumber"
            | "NameOfTheCompany"
            | "AddressOfRegisteredOfficeOfCompany"
            | "AddressOfCorporateOfficeOfCompany"
            | "EMailOfTheCompany"
            | "EmailOfTheCompany"
            | "TelephoneOfCompany"
            | "WebsiteOfCompany"
            | "ReportingBoundary"
            | "DateOfStartOfFinancialYear"
            | "DateOfEndOfFinancialYear"
            | "DateOfIncorporation"
            | "YearOfIncorporation"
            | "PaidUpValueOfShares"
            | "ValueOfSharesPaidUp"
            | "NameOfContactPerson"
            | "ContactNumberOfContactPerson"
            | "TelephoneOfContactPerson"
            | "EMailOfContactPerson"
            | "EmailOfContactPerson"
    )
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    let date_part = t.split('T').next().unwrap_or(t).trim();

    if let Ok(d) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(date_part, "%d-%m-%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(date_part, "%d/%m/%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(date_part, "%Y/%m/%d") {
        return Some(d);
    }
    if t.len() == 4 {
        if let Ok(year) = t.parse::<i32>() {
            return NaiveDate::from_ymd_opt(year, 1, 1);
        }
    }
    None
}
