// src/pipeline/geocoder.rs

use crate::storage::repository::entities::resolve_unmapped_pincodes_bulk;
use sqlx::PgPool;

pub struct GeocoderService {
    pool: PgPool,
}

impl GeocoderService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolves pending entities instantly in <10ms using local PostgreSQL pincode master
    pub async fn process_pending_batch(
        &self,
        _limit: i64,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let affected = resolve_unmapped_pincodes_bulk(&self.pool).await?;
        Ok(affected as usize)
    }
}
