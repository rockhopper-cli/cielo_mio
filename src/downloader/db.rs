// src/downloader/db.rs

use crate::downloader::types::{FileSyncStatus, PendingFile};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{PgPool, Row};
use std::time::Duration;

/// Connects to PostgreSQL with a timeout
pub async fn connect_to_database(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    db_name: &str,
    timeout_dur: Duration,
) -> Result<PgPool, String> {
    let connect_options = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .password(password)
        .database(db_name)
        .ssl_mode(PgSslMode::Disable);

    match tokio::time::timeout(
        timeout_dur,
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(timeout_dur)
            .connect_with(connect_options),
    )
    .await
    {
        Ok(Ok(pool)) => Ok(pool),
        Ok(Err(e)) => Err(format!("PostgreSQL connection error: {}", e)),
        Err(_) => Err(format!(
            "Connection timed out after {:.1}s",
            timeout_dur.as_secs_f32()
        )),
    }
}

#[derive(Clone)] 
pub struct DatabaseManager {
    pool: PgPool,
}

impl DatabaseManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Checks file_list to see if this file was already processed
    pub async fn check_file_status(&self, file_name: &str) -> Result<FileSyncStatus, sqlx::Error> {
        let query = "SELECT mapped, unmapped_reason FROM file_list WHERE file_name = $1 LIMIT 1";
        let record = sqlx::query(query)
            .bind(file_name)
            .fetch_optional(&self.pool)
            .await?;

        match record {
            Some(row) => {
                let mapped: bool = row.get("mapped");
                let reason: Option<String> = row.get("unmapped_reason");

                if mapped {
                    Ok(FileSyncStatus::AlreadyMapped)
                } else {
                    Ok(FileSyncStatus::FailedPreviously(reason))
                }
            }
            None => Ok(FileSyncStatus::NewFile),
        }
    }

    /// Records a newly downloaded file ready for Phase 3 parsing
    pub async fn record_success(&self, file: &PendingFile) -> Result<(), sqlx::Error> {
        let query = r#"
            INSERT INTO file_list (
                file_name, stock_market, upload_date, upload_time, file_type, mapped, unmapped_reason
            )
            VALUES ($1, $2, $3, $4, $5, FALSE, NULL)
            ON CONFLICT (file_name) 
            DO UPDATE SET 
                mapped = FALSE,
                unmapped_reason = NULL;
        "#;

        sqlx::query(query)
            .bind(&file.file_name)
            .bind(file.stock_market.as_str())
            .bind(file.upload_date)
            .bind(file.upload_time)
            .bind(file.file_type.as_str())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Records a failed download or corrupt stream in file_list
    pub async fn record_failure(
        &self,
        file: &PendingFile,
        error_reason: &str,
    ) -> Result<(), sqlx::Error> {
        let query = r#"
            INSERT INTO file_list (
                file_name, stock_market, upload_date, upload_time, file_type, mapped, unmapped_reason
            )
            VALUES ($1, $2, $3, $4, $5, FALSE, $6)
            ON CONFLICT (file_name) 
            DO UPDATE SET 
                mapped = FALSE,
                unmapped_reason = EXCLUDED.unmapped_reason;
        "#;

        sqlx::query(query)
            .bind(&file.file_name)
            .bind(file.stock_market.as_str())
            .bind(file.upload_date)
            .bind(file.upload_time)
            .bind(file.file_type.as_str())
            .bind(error_reason)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
