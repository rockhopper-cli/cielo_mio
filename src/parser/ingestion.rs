// src/parser/ingestion.rs

use crate::parser::industry::IndustryRegistry;
use crate::parser::transform::{EnrichedFact, EntityProfile};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

pub struct IngestionManager {
    pool: PgPool,
    industry_registry: IndustryRegistry,
}

impl IngestionManager {
    pub fn new(pool: PgPool, industry_registry: IndustryRegistry) -> Self {
        Self {
            pool,
            industry_registry,
        }
    }

    pub async fn ingest_filing(
        &self,
        file_id: Uuid,
        stock_market: &str,
        profile: &EntityProfile,
        facts: &[EnrichedFact],
    ) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // 1. Idempotence: Clean up any prior partial/failed records for this file
        sqlx::query("DELETE FROM brsr_facts WHERE file_uuid = $1")
            .bind(file_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM entity_list WHERE file_uuid = $1")
            .bind(file_id)
            .execute(&mut *tx)
            .await?;

        // 2. Classify industry and clamp to DB length constraints
        let industry_info = self
            .industry_registry
            .classify_cin(&profile.corporate_id_number);

        let clean_nic: String = industry_info.nic_code.chars().take(5).collect();
        let clean_division: String = industry_info.division.chars().take(120).collect();
        let clean_sector: String = industry_info.sector.chars().take(80).collect();

        let paid_up_decimal = profile
            .paid_up_shares_value
            .and_then(Decimal::from_f64_retain);

        // 3. Insert corporate profile into entity_list
        let entity_row = sqlx::query(
            r#"
            INSERT INTO entity_list (
                section, principle_number, sub_section, friendly_group, stock_market,
                file_uuid, corporate_id_number, company_name, address_corporate_office,
                address_registered_office, contact_person_name, contact_person_phone,
                contact_person_email, company_email, company_telephone, website,
                reporting_boundary, financial_year_start, financial_year_end,
                incorporation_date, paid_up_shares_value,
                nic_code, industry_division, industry_sector
            )
            VALUES (
                'A', 0, 1, 'entity_list', $1,
                $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10, $11, $12,
                $13, $14, $15,
                $16, $17,
                $18, $19, $20
            )
            RETURNING entity_id;
            "#,
        )
        .bind(stock_market)
        .bind(file_id)
        .bind(&profile.corporate_id_number)
        .bind(&profile.company_name)
        .bind(&profile.address_corporate_office)
        .bind(&profile.address_registered_office)
        .bind(&profile.contact_person_name)
        .bind(&profile.contact_person_phone)
        .bind(&profile.contact_person_email)
        .bind(&profile.company_email)
        .bind(&profile.company_telephone)
        .bind(&profile.website)
        .bind(&profile.reporting_boundary)
        .bind(profile.financial_year_start)
        .bind(profile.financial_year_end)
        .bind(profile.incorporation_date)
        .bind(paid_up_decimal)
        .bind(&clean_nic)
        .bind(&clean_division)
        .bind(&clean_sector)
        .fetch_one(&mut *tx)
        .await?;

        let client_uuid: Uuid = entity_row.get("entity_id");

        // 4. Bulk insert facts into brsr_facts in chunks of 500
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
                let num_dec = fact.numeric_value.and_then(Decimal::from_f64_retain);
                let std_num_dec = fact
                    .standard_numeric_value
                    .and_then(Decimal::from_f64_retain);

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

            qb.build().execute(&mut *tx).await?;
        }

        // 5. Update sync state in file_list
        sqlx::query(
            "UPDATE file_list SET mapped = TRUE, unmapped_reason = NULL WHERE file_id = $1",
        )
        .bind(file_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(client_uuid)
    }

    pub async fn record_failure(&self, file_id: Uuid, error_reason: &str) {
        let _ = sqlx::query(
            "UPDATE file_list SET mapped = FALSE, unmapped_reason = $1 WHERE file_id = $2",
        )
        .bind(error_reason)
        .bind(file_id)
        .execute(&self.pool)
        .await;
    }
}
