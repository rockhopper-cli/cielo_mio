// src/pipeline/ingestion.rs

use crate::domain::entity::EntityProfile;
use crate::domain::facts::EnrichedFact;
use crate::domain::taxonomy::IndustryRegistry;
use crate::storage::repository::{entities, facts, files};
use sqlx::PgPool;
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
        facts_list: &[EnrichedFact],
    ) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // 1. Clear previous facts and entity profile for idempotency
        facts::clear_prior_file_data(&mut tx, file_id).await?;

        // 2. Insert entity profile (with Rust fallback industry metadata if trigger leaves it unclassified)
        let fallback_industry = self
            .industry_registry
            .classify_cin_or_name(&profile.corporate_id_number, &profile.company_name);

        let client_uuid = entities::insert_entity_profile(
            &mut tx,
            file_id,
            stock_market,
            profile,
            Some(&fallback_industry),
        )
        .await?;

        // 3. Bulk insert facts
        facts::insert_facts_batch(&mut tx, file_id, client_uuid, facts_list).await?;

        // 4. Update sync state machine
        files::mark_file_mapped(&mut tx, file_id).await?;

        tx.commit().await?;
        Ok(client_uuid)
    }

    pub async fn record_failure(&self, file_id: Uuid, error_reason: &str) {
        files::record_ingestion_failure(&self.pool, file_id, error_reason).await;
    }
}
