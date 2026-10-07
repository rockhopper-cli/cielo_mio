// src/app/tab_analytics/data.rs

use crate::app::state::CompanyEsgMetrics;
use crate::parser::units::parse_number;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
struct EsgPriorityTracker {
    scope_1: u8,
    scope_2: u8,
    scope_3: u8,
    renewable: u8,
    water: u8,
    energy_int: u8,
    waste_int: u8,
    female_board: u8,
    female_wages: u8,
    turnover: u8,
    injuries: u8,
    fatalities: u8,
    female_workforce: u8,
    posh_upheld: u8,
    total_posh: u8,
    complaints_filed: u8,
    pending_complaints: u8,
    disciplinary: u8,
    wellbeing: u8,
}

#[inline]
fn determine_context_priority(ctx: &str) -> u8 {
    let l = ctx.to_ascii_lowercase();
    if l.contains("py") || l.contains("previous") || l.contains("prior") {
        1 // Prior Year (fallback only)
    } else if l.contains("cy") || l.contains("current") || l.contains("main") || l.contains("dcy") {
        3 // Verified Current Year
    } else {
        2 // General / Neutral context
    }
}

#[inline]
fn update_metric(
    target: &mut Option<f64>,
    target_p: &mut u8,
    new_val: f64,
    new_p: u8,
    prefer_larger_within_tier: bool,
) {
    if new_p > *target_p {
        *target = Some(new_val);
        *target_p = new_p;
    } else if new_p == *target_p {
        if target.is_none() || (prefer_larger_within_tier && new_val > target.unwrap()) {
            *target = Some(new_val);
        }
    }
}

#[inline]
fn update_pct_metric(target: &mut Option<f64>, target_p: &mut u8, new_val: f64, new_p: u8) {
    let clamped = new_val.clamp(0.0, 100.0);
    update_metric(target, target_p, clamped, new_p, false);
}

pub async fn fetch_esg_metrics_from_db(
    pool: &PgPool,
) -> Result<Vec<CompanyEsgMetrics>, sqlx::Error> {
    let entity_sql = r#"
        SELECT 
            entity_id, company_name, corporate_id_number,
            COALESCE(industry_sector, 'Diversified') as industry_sector,
            COALESCE(industry_division, 'General') as industry_division,
            financial_year_start, financial_year_end, paid_up_shares_value
        FROM entity_list
        ORDER BY company_name ASC, financial_year_start ASC;
    "#;

    let entity_rows = sqlx::query(entity_sql).fetch_all(pool).await?;
    let mut metrics_map: HashMap<Uuid, (CompanyEsgMetrics, EsgPriorityTracker)> =
        HashMap::with_capacity(entity_rows.len());

    for r in entity_rows {
        let ent_id: Uuid = r.get("entity_id");
        let name: String = r.get("company_name");
        let cin: String = r.get("corporate_id_number");
        let sec: String = r.get("industry_sector");
        let div: String = r.get("industry_division");
        let start: Option<chrono::NaiveDate> = r.get("financial_year_start");
        let end: Option<chrono::NaiveDate> = r.get("financial_year_end");
        let paid_up: Option<rust_decimal::Decimal> = r.get("paid_up_shares_value");

        let fy_label = match (start, end) {
            (Some(s), Some(e)) => format!("{} → {}", s, e),
            _ => "FY 2023-24".to_string(),
        };

        let paid_up_cr = paid_up.map(|d| {
            let num: f64 = d.to_string().parse().unwrap_or(0.0);
            num / 10_000_000.0
        });

        metrics_map.insert(
            ent_id,
            (
                CompanyEsgMetrics {
                    entity_id: ent_id,
                    company_name: name,
                    corporate_id_number: cin,
                    industry_sector: sec,
                    industry_division: div,
                    fy_start: start,
                    fy_end: end,
                    fy_label,
                    paid_up_capital_cr: paid_up_cr,
                    scope_1_tco2e: None,
                    scope_2_tco2e: None,
                    scope_3_tco2e: None,
                    water_withdrawal_kl: None,
                    energy_intensity: None,
                    waste_intensity: None,
                    female_board_directors_pct: None,
                    gross_wages_to_female_pct: None,
                    employee_turnover_pct: None,
                    work_related_injuries: None,
                    fatalities: None,
                    female_workforce_pct: None,
                    posh_complaints_upheld: None,
                    total_posh_complaints: None,
                    complaints_filed: None,
                    pending_complaints: None,
                    disciplinary_actions: None,
                    cost_on_wellbeing_pct: None,
                    renewable_energy_pct: None,
                },
                EsgPriorityTracker::default(),
            ),
        );
    }

    // Comprehensive retrieval ensuring unmapped or differently-grouped facts are never excluded
    let facts_sql = r#"
        SELECT 
            client_uuid,
            COALESCE(friendly_grouping, '') as friendly_grouping,
            standard_indicator_capmkt,
            indicator_capmkt,
            COALESCE(standard_context_reference, context_reference, '') as context_ref,
            standard_numeric_value,
            numeric_value,
            raw_value
        FROM brsr_facts
        WHERE standard_indicator_capmkt ILIKE ANY (ARRAY[
            '%scope%', '%emission%', '%water%', '%energy%', '%renewable%',
            '%waste%', '%female%', '%women%', '%turnover%', '%injur%',
            '%fatalit%', '%posh%', '%complaint%', '%disciplinary%',
            '%wellbeing%', '%well-being%', '%director%', '%wage%', '%workforce%'
        ])
        OR friendly_grouping ILIKE ANY (ARRAY[
            '%ghg%', '%water%', '%energy%', '%waste%', '%women%', '%turnover%',
            '%safety%', '%posh%', '%complaint%', '%remuneration%', '%benefit%',
            '%grievance%', '%employee%'
        ]);
    "#;

    let fact_rows = sqlx::query(facts_sql).fetch_all(pool).await?;

    for r in fact_rows {
        let ent_id: Uuid = r.get("client_uuid");
        let group: String = r.get("friendly_grouping");
        let std_tag: String = r.get("standard_indicator_capmkt");
        let in_tag: String = r.get("indicator_capmkt");
        let ctx_ref: String = r.get("context_ref");
        let std_num: Option<rust_decimal::Decimal> = r.get("standard_numeric_value");
        let num: Option<rust_decimal::Decimal> = r.get("numeric_value");
        let raw: String = r.get("raw_value");

        let val_num: Option<f64> = std_num
            .or(num)
            .and_then(|d| d.to_string().parse().ok())
            .or_else(|| parse_number(&raw));

        let v = match val_num {
            Some(val) => val,
            None => continue,
        };

        let prio = determine_context_priority(&ctx_ref);
        let std_lower = std_tag.to_lowercase();
        let in_lower = in_tag.to_lowercase();
        let group_lower = group.to_lowercase();

        let tag_matches = |pat: &str| {
            std_lower.contains(pat) || in_lower.contains(pat) || group_lower.contains(pat)
        };

        if let Some((entry, tracker)) = metrics_map.get_mut(&ent_id) {
            // Scope 1
            if (tag_matches("scope 1")
                || tag_matches("scope1")
                || (group_lower.contains("ghg") && tag_matches("1")))
                && !tag_matches("intensity")
            {
                update_metric(
                    &mut entry.scope_1_tco2e,
                    &mut tracker.scope_1,
                    v,
                    prio,
                    true,
                );
            }
            // Scope 2
            else if (tag_matches("scope 2")
                || tag_matches("scope2")
                || (group_lower.contains("ghg") && tag_matches("2")))
                && !tag_matches("intensity")
            {
                update_metric(
                    &mut entry.scope_2_tco2e,
                    &mut tracker.scope_2,
                    v,
                    prio,
                    true,
                );
            }
            // Scope 3
            else if tag_matches("scope 3")
                || tag_matches("scope3")
                || group_lower.contains("scope3")
            {
                update_metric(
                    &mut entry.scope_3_tco2e,
                    &mut tracker.scope_3,
                    v,
                    prio,
                    true,
                );
            }
            // Renewable Energy %
            else if tag_matches("renewable")
                && (tag_matches("percent") || tag_matches("share") || tag_matches("ratio"))
            {
                update_pct_metric(
                    &mut entry.renewable_energy_pct,
                    &mut tracker.renewable,
                    v,
                    prio,
                );
            }
            // Water Withdrawal
            else if (tag_matches("water") || group_lower.contains("water"))
                && (tag_matches("withdrawal") || tag_matches("consumption"))
            {
                update_metric(
                    &mut entry.water_withdrawal_kl,
                    &mut tracker.water,
                    v,
                    prio,
                    true,
                );
            }
            // Energy Intensity
            else if (tag_matches("energy") || group_lower.contains("energy"))
                && tag_matches("intensity")
            {
                update_metric(
                    &mut entry.energy_intensity,
                    &mut tracker.energy_int,
                    v,
                    prio,
                    false,
                );
            }
            // Waste Intensity
            else if (tag_matches("waste") || group_lower.contains("waste"))
                && tag_matches("intensity")
            {
                update_metric(
                    &mut entry.waste_intensity,
                    &mut tracker.waste_int,
                    v,
                    prio,
                    false,
                );
            }
            // Female Board Directors %
            else if (tag_matches("female")
                || tag_matches("women")
                || group_lower.contains("women"))
                && (tag_matches("director") || tag_matches("board"))
            {
                update_pct_metric(
                    &mut entry.female_board_directors_pct,
                    &mut tracker.female_board,
                    v,
                    prio,
                );
            }
            // Gross Wages to Female %
            else if (tag_matches("wages")
                || tag_matches("remuneration")
                || group_lower.contains("remuneration"))
                && (tag_matches("female") || tag_matches("women"))
            {
                update_pct_metric(
                    &mut entry.gross_wages_to_female_pct,
                    &mut tracker.female_wages,
                    v,
                    prio,
                );
            }
            // Employee Turnover %
            else if tag_matches("turnover") || group_lower.contains("turnover") {
                update_pct_metric(
                    &mut entry.employee_turnover_pct,
                    &mut tracker.turnover,
                    v,
                    prio,
                );
            }
            // Work-Related Injuries
            else if tag_matches("injur")
                || (group_lower.contains("safety") && tag_matches("incident"))
            {
                update_metric(
                    &mut entry.work_related_injuries,
                    &mut tracker.injuries,
                    v,
                    prio,
                    true,
                );
            }
            // Fatalities
            else if tag_matches("fatal") {
                update_metric(
                    &mut entry.fatalities,
                    &mut tracker.fatalities,
                    v,
                    prio,
                    true,
                );
            }
            // Female Workforce %
            else if (tag_matches("female")
                || tag_matches("women")
                || group_lower.contains("women"))
                && (tag_matches("workforce") || tag_matches("employee"))
            {
                update_pct_metric(
                    &mut entry.female_workforce_pct,
                    &mut tracker.female_workforce,
                    v,
                    prio,
                );
            }
            // POSH Complaints Upheld
            else if (tag_matches("posh") || group_lower.contains("posh")) && tag_matches("upheld")
            {
                update_metric(
                    &mut entry.posh_complaints_upheld,
                    &mut tracker.posh_upheld,
                    v,
                    prio,
                    true,
                );
            }
            // Total POSH Complaints
            else if tag_matches("posh") || group_lower.contains("posh") {
                update_metric(
                    &mut entry.total_posh_complaints,
                    &mut tracker.total_posh,
                    v,
                    prio,
                    true,
                );
            }
            // Complaints Filed
            else if (tag_matches("complaint") || group_lower.contains("grievance"))
                && tag_matches("filed")
            {
                update_metric(
                    &mut entry.complaints_filed,
                    &mut tracker.complaints_filed,
                    v,
                    prio,
                    true,
                );
            }
            // Pending Complaints
            else if (tag_matches("complaint") || group_lower.contains("grievance"))
                && tag_matches("pending")
            {
                update_metric(
                    &mut entry.pending_complaints,
                    &mut tracker.pending_complaints,
                    v,
                    prio,
                    true,
                );
            }
            // Disciplinary Actions
            else if tag_matches("disciplinary") || group_lower.contains("conduct") {
                update_metric(
                    &mut entry.disciplinary_actions,
                    &mut tracker.disciplinary,
                    v,
                    prio,
                    true,
                );
            }
            // Cost on Well-Being %
            else if tag_matches("wellbeing")
                || tag_matches("well-being")
                || (group_lower.contains("benefit") && tag_matches("cost"))
            {
                update_metric(
                    &mut entry.cost_on_wellbeing_pct,
                    &mut tracker.wellbeing,
                    v,
                    prio,
                    false,
                );
            }
        }
    }

    let mut result: Vec<CompanyEsgMetrics> = metrics_map.into_values().map(|(m, _)| m).collect();

    result.sort_by(|a, b| {
        a.company_name
            .cmp(&b.company_name)
            .then(a.fy_start.cmp(&b.fy_start))
            .then(a.fy_end.cmp(&b.fy_end))
    });

    Ok(result)
}
