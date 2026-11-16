// src/pipeline/xbrl/transform.rs

use crate::domain::entity::EntityProfile;
use crate::domain::facts::{EnrichedFact, ParsedFiling};
use crate::domain::taxonomy::FormRule;
use crate::domain::units::parse_decimal;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

pub struct FormDictionary {
    rules: HashMap<(String, String), FormRule>,
    increment_by_tag: HashMap<String, Vec<FormRule>>,
    tag_fallback: HashMap<String, FormRule>,
}

impl FormDictionary {
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                raw_tag AS indicator_capmkt,
                raw_context AS context_reference,
                std_tag AS standard_indicator_capmkt,
                std_context AS standard_context_reference,
                section,
                principle_number,
                sub_section,
                indicator_type,
                friendly_grouping,
                is_axis_increment AS contextref_increment
            FROM xbrl_tag_rules
            ORDER BY is_legacy ASC, rule_id ASC;
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut rules = HashMap::with_capacity(rows.len());
        let mut increment_by_tag: HashMap<String, Vec<FormRule>> = HashMap::new();
        let mut tag_fallback: HashMap<String, FormRule> = HashMap::new();

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
                standard_indicator_capmkt: std_indicator
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| indicator.clone()),
                standard_context_reference: std_context
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| context.clone()),
                section,
                principle_number: principle,
                sub_section: sub_sec,
                indicator_type: ind_type,
                friendly_grouping: friendly,
            };

            if inc {
                increment_by_tag
                    .entry(indicator.clone())
                    .or_default()
                    .push(rule.clone());
            }

            tag_fallback
                .entry(indicator.clone())
                .or_insert_with(|| rule.clone());
            rules.insert((indicator, context), rule);
        }

        Ok(Self {
            rules,
            increment_by_tag,
            tag_fallback,
        })
    }

    fn resolve_rule<'a>(
        &'a self,
        in_capmkt: &str,
        context_ref: &'a str,
    ) -> Option<(Cow<'a, FormRule>, Cow<'a, str>)> {
        let key = (in_capmkt.to_string(), context_ref.to_string());
        if let Some(exact) = self.rules.get(&key) {
            return Some((
                Cow::Borrowed(exact),
                Cow::Borrowed(exact.standard_context_reference.as_str()),
            ));
        }

        // Dynamic O(1) lookup by tag for contextref_increment axis cloning
        if let Some(candidates) = self.increment_by_tag.get(in_capmkt) {
            for inc_rule in candidates {
                let prefix = inc_rule
                    .context_reference
                    .trim_end_matches(|c: char| c.is_ascii_digit());
                if let Some(suffix) = context_ref.strip_prefix(prefix)
                    && !suffix.is_empty()
                    && suffix.chars().all(|c| c.is_ascii_digit())
                {
                    let std_prefix = inc_rule
                        .standard_context_reference
                        .trim_end_matches(|c: char| c.is_ascii_digit());
                    return Some((
                        Cow::Borrowed(inc_rule),
                        Cow::Owned(format!("{}{}", std_prefix, suffix)),
                    ));
                }
            }
        }

        // Tag-level routing fallback: prevents Prior Year / alternate context facts from misrouting into Section A
        if let Some(fb) = self.tag_fallback.get(in_capmkt) {
            return Some((Cow::Borrowed(fb), Cow::Borrowed(context_ref)));
        }

        None
    }

    pub fn enrich_facts(
        &self,
        filing: &ParsedFiling,
        file_symbol: Option<&str>,
        file_scrip: Option<&str>,
    ) -> (Vec<EnrichedFact>, EntityProfile) {
        let filing_cin = &filing.client_id;
        let facts = &filing.facts;

        let mut enriched = Vec::with_capacity(facts.len());
        let mut entity_map: HashMap<String, (u8, String)> = HashMap::new();
        let mut seen_facts: HashSet<(String, String, Option<String>, String)> = HashSet::new();

        for fact in facts {
            let clean_val = fact.raw_value.trim();
            if clean_val.is_empty() {
                continue;
            }

            let (section, principle, sub_sec, ind_type, std_tag, std_ctx, friendly) =
                match self.resolve_rule(&fact.in_capmkt, &fact.context_ref) {
                    Some((r, resolved_std_ctx)) => (
                        r.section.clone(),
                        r.principle_number,
                        r.sub_section,
                        r.indicator_type.clone(),
                        r.standard_indicator_capmkt.clone(),
                        resolved_std_ctx.into_owned(),
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

            let is_entity_group = friendly
                .as_ref()
                .map(|fg| fg.eq_ignore_ascii_case("entity_list"))
                .unwrap_or(false);

            if is_entity_group || is_profile_tag(&std_tag) || is_profile_tag(&fact.in_capmkt) {
                let priority = context_profile_priority(&fact.context_ref);
                insert_prioritized(&mut entity_map, &std_tag, priority, clean_val);
                insert_prioritized(&mut entity_map, &fact.in_capmkt, priority, clean_val);
            }

            let dedupe_key = (
                std_tag.clone(),
                std_ctx.clone(),
                fact.unit_ref.clone(),
                clean_val.to_string(),
            );
            if !seen_facts.insert(dedupe_key) {
                continue;
            }

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
                numeric_value: fact.numeric_value,
                unit_reference: fact.unit_ref.clone(),
                standard_unit_reference: final_std_unit,
                decimals: fact.decimals,
                standard_raw_value: fact
                    .standard_value_str
                    .clone()
                    .or_else(|| fact.standard_value.map(|v| v.normalize().to_string()))
                    .or_else(|| Some(clean_val.to_string())),
                standard_numeric_value: fact.standard_value,
            });
        }

        let resolved_cin = take_profile(&mut entity_map, "CorporateIdentityNumber")
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| filing_cin.to_string());

        let resolved_isin = take_profile(&mut entity_map, "ISIN")
            .or_else(|| take_profile(&mut entity_map, "ISINNumber"))
            .or_else(|| {
                take_profile(
                    &mut entity_map,
                    "InternationalSecuritiesIdentificationNumber",
                )
            })
            .or_else(|| filing.detected_isin.clone())
            .map(|s| s.trim().to_uppercase())
            .filter(|s| s.len() == 12 && s.starts_with("IN"));

        let resolved_symbol = take_profile(&mut entity_map, "NSESymbol")
            .or_else(|| take_profile(&mut entity_map, "Symbol"))
            .or_else(|| take_profile(&mut entity_map, "TradingSymbol"))
            .or_else(|| file_symbol.map(str::to_string))
            .or_else(|| filing.detected_symbol.clone())
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());

        let resolved_scrip = take_profile(&mut entity_map, "BSEScripCode")
            .or_else(|| take_profile(&mut entity_map, "ScripCode"))
            .or_else(|| file_scrip.map(str::to_string))
            .or_else(|| filing.detected_scrip_code.clone())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let profile = EntityProfile {
            corporate_id_number: resolved_cin,
            isin: resolved_isin,
            nse_symbol: resolved_symbol,
            bse_scrip_code: resolved_scrip,
            company_name: take_profile(&mut entity_map, "NameOfTheCompany")
                .or_else(|| take_profile(&mut entity_map, "CompanyName"))
                .unwrap_or_else(|| "Unknown Company".to_string()),
            address_registered_office: take_profile(
                &mut entity_map,
                "AddressOfRegisteredOfficeOfCompany",
            )
            .or_else(|| take_profile(&mut entity_map, "RegisteredOfficeAddress")),
            address_corporate_office: take_profile(
                &mut entity_map,
                "AddressOfCorporateOfficeOfCompany",
            )
            .or_else(|| take_profile(&mut entity_map, "CorporateOfficeAddress")),
            company_email: take_profile(&mut entity_map, "EMailOfTheCompany")
                .or_else(|| take_profile(&mut entity_map, "EmailOfTheCompany")),
            company_telephone: take_profile(&mut entity_map, "TelephoneOfCompany")
                .or_else(|| take_profile(&mut entity_map, "TelephoneNumber")),
            website: take_profile(&mut entity_map, "WebsiteOfCompany")
                .or_else(|| take_profile(&mut entity_map, "Website")),
            reporting_boundary: take_profile(&mut entity_map, "ReportingBoundary")
                .map(|raw| normalize_reporting_boundary(&raw)),
            financial_year_start: get_profile(&entity_map, "DateOfStartOfFinancialYear")
                .or_else(|| get_profile(&entity_map, "FinancialYearStartDate"))
                .and_then(parse_date),
            financial_year_end: get_profile(&entity_map, "DateOfEndOfFinancialYear")
                .or_else(|| get_profile(&entity_map, "FinancialYearEndDate"))
                .and_then(parse_date),
            incorporation_date: get_profile(&entity_map, "DateOfIncorporation")
                .or_else(|| get_profile(&entity_map, "YearOfIncorporation"))
                .and_then(parse_date),
            paid_up_shares_value: get_profile(&entity_map, "ValueOfSharesPaidUp")
                .or_else(|| get_profile(&entity_map, "PaidUpValueOfShares"))
                .and_then(parse_decimal),
            contact_person_name: take_profile(&mut entity_map, "NameOfContactPerson"),
            contact_person_phone: take_profile(&mut entity_map, "ContactNumberOfContactPerson")
                .or_else(|| take_profile(&mut entity_map, "TelephoneOfContactPerson")),
            contact_person_email: take_profile(&mut entity_map, "EMailOfContactPerson")
                .or_else(|| take_profile(&mut entity_map, "EmailOfContactPerson")),
        };

        (enriched, profile)
    }
}

fn normalize_reporting_boundary(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("consolidat") {
        "Consolidated".to_string()
    } else if lower.contains("standalone") {
        "Standalone".to_string()
    } else {
        trimmed.chars().take(50).collect()
    }
}

fn context_profile_priority(ctx: &str) -> u8 {
    let lo = ctx.to_ascii_lowercase();
    if lo == "dcymain" || lo == "icymain" || lo == "main" {
        3
    } else if lo.contains("py") || lo.contains("previous") || lo.contains("prior") {
        1
    } else {
        2
    }
}

fn insert_prioritized(map: &mut HashMap<String, (u8, String)>, key: &str, priority: u8, val: &str) {
    match map.get_mut(key) {
        Some((existing_prio, existing_val)) => {
            if priority >= *existing_prio {
                *existing_prio = priority;
                *existing_val = val.to_string();
            }
        }
        None => {
            map.insert(key.to_string(), (priority, val.to_string()));
        }
    }
}

fn take_profile(map: &mut HashMap<String, (u8, String)>, key: &str) -> Option<String> {
    map.remove(key).map(|(_, v)| v)
}

fn get_profile<'a>(map: &'a HashMap<String, (u8, String)>, key: &str) -> Option<&'a str> {
    map.get(key).map(|(_, v)| v.as_str())
}

fn is_profile_tag(tag: &str) -> bool {
    matches!(
        tag,
        "CorporateIdentityNumber"
            | "ISIN"
            | "ISINNumber"
            | "InternationalSecuritiesIdentificationNumber"
            | "Symbol"
            | "NSESymbol"
            | "TradingSymbol"
            | "ScripCode"
            | "BSEScripCode"
            | "NameOfTheCompany"
            | "CompanyName"
            | "AddressOfRegisteredOfficeOfCompany"
            | "RegisteredOfficeAddress"
            | "AddressOfCorporateOfficeOfCompany"
            | "CorporateOfficeAddress"
            | "EMailOfTheCompany"
            | "EmailOfTheCompany"
            | "TelephoneOfCompany"
            | "TelephoneNumber"
            | "WebsiteOfCompany"
            | "Website"
            | "ReportingBoundary"
            | "DateOfStartOfFinancialYear"
            | "FinancialYearStartDate"
            | "DateOfEndOfFinancialYear"
            | "FinancialYearEndDate"
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
    if t.len() == 4
        && let Ok(year) = t.parse::<i32>()
    {
        return NaiveDate::from_ymd_opt(year, 1, 1);
    }
    None
}
