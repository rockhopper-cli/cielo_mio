// src/storage/repository/facts.rs

use crate::domain::facts::EnrichedFact;
use crate::ui::state::FormFactRow;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

/// Fetches all disclosures mapped to an entity for the Form Explorer view
pub async fn fetch_facts_by_entity(
    pool: &PgPool,
    entity_id: Uuid,
) -> Result<Vec<FormFactRow>, sqlx::Error> {
    let sql = r#"
        SELECT 
            section, principle_number, sub_section, indicator_type,
            standard_indicator_capmkt, standard_context_reference,
            friendly_grouping,
            COALESCE(raw_value, '') as raw_value,
            standard_numeric_value, standard_unit_reference
        FROM brsr_facts
        WHERE client_uuid = $1;
    "#;

    let rows = sqlx::query(sql).bind(entity_id).fetch_all(pool).await?;
    let mut facts = Vec::with_capacity(rows.len());

    for r in rows {
        let p_num = r
            .try_get::<i16, _>("principle_number")
            .or_else(|_| r.try_get::<i32, _>("principle_number").map(|v| v as i16))
            .unwrap_or(0);

        let sub_sec = r
            .try_get::<i16, _>("sub_section")
            .or_else(|_| r.try_get::<i32, _>("sub_section").map(|v| v as i16))
            .unwrap_or(0);

        facts.push(FormFactRow {
            section: r.get("section"),
            principle_number: p_num,
            sub_section: sub_sec,
            indicator_type: r.get("indicator_type"),
            standard_indicator: r.get("standard_indicator_capmkt"),
            context_ref: r.get("standard_context_reference"),
            friendly_grouping: r.get("friendly_grouping"),
            raw_value: r.get("raw_value"),
            standard_numeric_value: r.get("standard_numeric_value"),
            standard_unit: r.get("standard_unit_reference"),
        });
    }

    Ok(facts)
}

/// Clears previous facts and entities for a file to enforce idempotency
pub async fn clear_prior_file_data(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    file_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM brsr_facts WHERE file_uuid = $1")
        .bind(file_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query("DELETE FROM entity_list WHERE file_uuid = $1")
        .bind(file_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

#[inline]
fn clamp_numeric_20_4(val: Option<Decimal>) -> Option<Decimal> {
    let limit = Decimal::from(10_000_000_000_000_000_i64); // fits NUMERIC(20, 4)
    val.filter(|d| d.abs() < limit).map(|d| d.round_dp(4))
}

/// Bulk inserts facts in 500-item chunks using QueryBuilder
pub async fn insert_facts_batch(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    file_id: Uuid,
    client_uuid: Uuid,
    facts: &[EnrichedFact],
) -> Result<(), sqlx::Error> {
    for chunk in facts.chunks(500) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
            INSERT INTO brsr_facts (
                section, principle_number, indicator_type, sub_section, client_id,
                file_uuid, client_uuid, indicator_capmkt, standard_indicator_capmkt,
                context_reference, standard_context_reference, friendly_grouping,
                response_datatype, raw_value, numeric_value, unit_reference,
                standard_unit_reference, decimals, standard_raw_value, standard_numeric_value
            ) 
            "#,
        );

        qb.push_values(chunk, |mut b, fact| {
            let num_dec = clamp_numeric_20_4(fact.numeric_value);
            let std_num_dec = fact.standard_numeric_value;

            b.push_bind(&fact.section)
                .push_bind(fact.principle_number)
                .push_bind(&fact.indicator_type)
                .push_bind(fact.sub_section)
                .push_bind(&fact.client_id)
                .push_bind(file_id)
                .push_bind(client_uuid)
                .push_bind(&fact.indicator_capmkt)
                .push_bind(&fact.standard_indicator_capmkt)
                .push_bind(&fact.context_reference)
                .push_bind(&fact.standard_context_reference)
                .push_bind(&fact.friendly_grouping)
                .push_bind(fact.response_datatype)
                .push_bind(&fact.raw_value)
                .push_bind(num_dec)
                .push_bind(&fact.unit_reference)
                .push_bind(&fact.standard_unit_reference)
                .push_bind(fact.decimals)
                .push_bind(&fact.standard_raw_value)
                .push_bind(std_num_dec);
        });

        qb.build().execute(&mut **tx).await?;
    }

    Ok(())
}
