// src/storage/repository/analytics.rs

use crate::ui::state::CompanyEsgMetrics;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

// Cache layout: sector -> sql_column_name (e.g. "median_scope_1") -> median_value
pub static SECTOR_BENCHMARKS: LazyLock<RwLock<HashMap<String, HashMap<String, f64>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn fetch_esg_metrics(pool: &PgPool) -> Result<Vec<CompanyEsgMetrics>, sqlx::Error> {
    let sql = r#"
        SELECT 
            entity_id,
            company_name,
            corporate_id_number,
            industry_sector,
            industry_division,
            financial_year_start,
            financial_year_end,
            paid_up_capital_cr,
            scope_1_tco2e,
            scope_2_tco2e,
            scope_3_tco2e,
            water_withdrawal_kl,
            energy_intensity,
            waste_intensity,
            renewable_energy_pct,
            female_board_directors_pct,
            gross_wages_to_female_pct,
            employee_turnover_pct,
            work_related_injuries,
            fatalities,
            female_workforce_pct,
            posh_complaints_upheld,
            total_posh_complaints,
            complaints_filed,
            pending_complaints,
            disciplinary_actions,
            cost_on_wellbeing_pct
        FROM view_company_esg_kpis
        ORDER BY company_name ASC, financial_year_start ASC NULLS LAST;
    "#;

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    let mut metrics = Vec::with_capacity(rows.len());

    for r in rows {
        let start: Option<chrono::NaiveDate> = r.get("financial_year_start");
        let end: Option<chrono::NaiveDate> = r.get("financial_year_end");

        let fy_label = match (start, end) {
            (Some(s), Some(e)) => format!("{} → {}", s, e),
            _ => "Period N/A".to_string(),
        };

        let to_f64 = |col: &str| -> Option<f64> {
            r.try_get::<Option<Decimal>, _>(col)
                .ok()
                .flatten()
                .and_then(|d| d.to_f64())
                .or_else(|| r.try_get::<Option<f64>, _>(col).ok().flatten())
        };

        let sector: String = r
            .try_get("industry_sector")
            .unwrap_or_else(|_| "Unclassified".to_string());
        let division: String = r
            .try_get("industry_division")
            .unwrap_or_else(|_| "General".to_string());

        metrics.push(CompanyEsgMetrics {
            entity_id: r.get("entity_id"),
            company_name: r.get("company_name"),
            corporate_id_number: r.get("corporate_id_number"),
            industry_sector: sector,
            industry_division: division,
            fy_start: start,
            fy_end: end,
            fy_label,
            paid_up_capital_cr: to_f64("paid_up_capital_cr"),
            scope_1_tco2e: to_f64("scope_1_tco2e"),
            scope_2_tco2e: to_f64("scope_2_tco2e"),
            scope_3_tco2e: to_f64("scope_3_tco2e"),
            water_withdrawal_kl: to_f64("water_withdrawal_kl"),
            energy_intensity: to_f64("energy_intensity"),
            waste_intensity: to_f64("waste_intensity"),
            female_board_directors_pct: to_f64("female_board_directors_pct"),
            gross_wages_to_female_pct: to_f64("gross_wages_to_female_pct"),
            employee_turnover_pct: to_f64("employee_turnover_pct"),
            work_related_injuries: to_f64("work_related_injuries"),
            fatalities: to_f64("fatalities"),
            female_workforce_pct: to_f64("female_workforce_pct"),
            posh_complaints_upheld: to_f64("posh_complaints_upheld"),
            total_posh_complaints: to_f64("total_posh_complaints"),
            complaints_filed: to_f64("complaints_filed"),
            pending_complaints: to_f64("pending_complaints"),
            disciplinary_actions: to_f64("disciplinary_actions"),
            cost_on_wellbeing_pct: to_f64("cost_on_wellbeing_pct"),
            renewable_energy_pct: to_f64("renewable_energy_pct"),
        });
    }

    // Compute exact percentile_cont(0.5) sector medians directly from the fetched metrics (avoids scanning brsr_facts twice)
    update_sector_benchmarks_from_metrics(&metrics);

    Ok(metrics)
}

fn median_of(vals: &mut [f64]) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.total_cmp(b));
    let n = vals.len();
    if n % 2 == 1 {
        Some(vals[n / 2])
    } else {
        Some((vals[n / 2 - 1] + vals[n / 2]) / 2.0)
    }
}

type MetricExtractor = (&'static str, fn(&CompanyEsgMetrics) -> Option<f64>);

fn update_sector_benchmarks_from_metrics(metrics: &[CompanyEsgMetrics]) {
    let mut by_sector: HashMap<&str, Vec<&CompanyEsgMetrics>> = HashMap::new();
    for m in metrics {
        by_sector
            .entry(m.industry_sector.as_str())
            .or_default()
            .push(m);
    }

    let extractors: [MetricExtractor; 19] = [
        ("median_scope_1", |m| m.scope_1_tco2e),
        ("median_scope_2", |m| m.scope_2_tco2e),
        ("median_scope_3", |m| m.scope_3_tco2e),
        ("median_water", |m| m.water_withdrawal_kl),
        ("median_energy_intensity", |m| m.energy_intensity),
        ("median_waste_intensity", |m| m.waste_intensity),
        ("median_renewable_energy_pct", |m| m.renewable_energy_pct),
        ("median_female_board_pct", |m| m.female_board_directors_pct),
        ("median_gross_wages_pct", |m| m.gross_wages_to_female_pct),
        ("median_turnover_pct", |m| m.employee_turnover_pct),
        ("median_injuries", |m| m.work_related_injuries),
        ("median_fatalities", |m| m.fatalities),
        ("median_female_workforce_pct", |m| m.female_workforce_pct),
        ("median_posh_upheld", |m| m.posh_complaints_upheld),
        ("median_total_posh", |m| m.total_posh_complaints),
        ("median_complaints_filed", |m| m.complaints_filed),
        ("median_pending_complaints", |m| m.pending_complaints),
        ("median_disciplinary_actions", |m| m.disciplinary_actions),
        ("median_cost_wellbeing_pct", |m| m.cost_on_wellbeing_pct),
    ];

    let mut cache = HashMap::with_capacity(by_sector.len());
    for (sector, items) in by_sector {
        let mut medians = HashMap::with_capacity(extractors.len());
        for (col_name, extractor) in extractors {
            let mut vals: Vec<f64> = items.iter().filter_map(|m| extractor(m)).collect();
            if let Some(med) = median_of(&mut vals) {
                medians.insert(col_name.to_string(), med);
            }
        }
        cache.insert(sector.to_string(), medians);
    }

    if let Ok(mut lock) = SECTOR_BENCHMARKS.write() {
        *lock = cache;
    }
}

/// Fetches raw tabular facts for CSV export
pub async fn fetch_export_facts(
    pool: &PgPool,
    cin_filter: &str,
    sec_filter: &str,
    grp_filter: &str,
) -> Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let sql = r#"
        SELECT 
            e.company_name, e.corporate_id_number, COALESCE(e.industry_sector, 'Unclassified') as industry_sector,
            f.section, f.principle_number, f.indicator_type,
            COALESCE(f.friendly_grouping, 'general') as friendly_grouping,
            f.standard_indicator_capmkt,
            COALESCE(f.standard_context_reference, f.context_reference, '') as context_ref,
            f.raw_value, f.standard_numeric_value, f.standard_unit_reference,
            f.response_datatype
        FROM brsr_facts f
        JOIN entity_list e ON f.client_uuid = e.entity_id
        WHERE ($1 = '' OR e.corporate_id_number = $1)
          AND ($2 = '' OR e.industry_sector = $2)
          AND ($3 = '' OR f.friendly_grouping = $3)
        ORDER BY e.company_name, f.section, f.principle_number, f.standard_indicator_capmkt;
    "#;

    sqlx::query(sql)
        .bind(cin_filter)
        .bind(sec_filter)
        .bind(grp_filter)
        .fetch_all(pool)
        .await
}
