// src/storage/repository/files.rs

use crate::domain::download::{FileSyncStatus, PendingFile};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Checks file_list to see if this file was already downloaded and processed
pub async fn check_file_status(
    pool: &PgPool,
    file_name: &str,
) -> Result<FileSyncStatus, sqlx::Error> {
    let query = "SELECT mapped, unmapped_reason FROM file_list WHERE file_name = $1 LIMIT 1";
    let record = sqlx::query(query)
        .bind(file_name)
        .fetch_optional(pool)
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

/// Records a newly downloaded file ready for parsing
pub async fn record_file_success(pool: &PgPool, file: &PendingFile) -> Result<(), sqlx::Error> {
    let query = r#"
        INSERT INTO file_list (
            file_name, stock_market, symbol, scrip_code, upload_date, upload_time, file_type, mapped, unmapped_reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, NULL)
        ON CONFLICT (file_name) 
        DO UPDATE SET 
            mapped = FALSE,
            unmapped_reason = NULL,
            symbol = COALESCE(EXCLUDED.symbol, file_list.symbol),
            scrip_code = COALESCE(EXCLUDED.scrip_code, file_list.scrip_code);
    "#;

    sqlx::query(query)
        .bind(&file.file_name)
        .bind(file.stock_market.as_str())
        .bind(&file.symbol)
        .bind(&file.scrip_code)
        .bind(file.upload_date)
        .bind(file.upload_time)
        .bind(file.file_type.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

/// Records a failed download or corrupt stream in file_list
pub async fn record_file_failure(
    pool: &PgPool,
    file: &PendingFile,
    error_reason: &str,
) -> Result<(), sqlx::Error> {
    let query = r#"
        INSERT INTO file_list (
            file_name, stock_market, symbol, scrip_code, upload_date, upload_time, file_type, mapped, unmapped_reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8)
        ON CONFLICT (file_name) 
        DO UPDATE SET 
            mapped = FALSE,
            unmapped_reason = EXCLUDED.unmapped_reason,
            symbol = COALESCE(EXCLUDED.symbol, file_list.symbol),
            scrip_code = COALESCE(EXCLUDED.scrip_code, file_list.scrip_code);
    "#;

    sqlx::query(query)
        .bind(&file.file_name)
        .bind(file.stock_market.as_str())
        .bind(&file.symbol)
        .bind(&file.scrip_code)
        .bind(file.upload_date)
        .bind(file.upload_time)
        .bind(file.file_type.as_str())
        .bind(error_reason)
        .execute(pool)
        .await?;

    Ok(())
}

/// Fetches a batch of unmapped XML filing headers for processing
pub async fn fetch_pending_xml_batch(
    pool: &PgPool,
    batch_size: i64,
) -> Result<Vec<(Uuid, String, String, Option<String>, Option<String>)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT file_id, file_name, stock_market, symbol, scrip_code FROM file_list \
         WHERE file_type = 'XML' AND mapped = FALSE AND unmapped_reason IS NULL \
         ORDER BY upload_date DESC, file_name ASC \
         LIMIT $1",
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for r in rows {
        result.push((
            r.get("file_id"),
            r.get("file_name"),
            r.get("stock_market"),
            r.get("symbol"),
            r.get("scrip_code"),
        ));
    }
    Ok(result)
}

/// Resets error state on failed filings to enable re-ingestion
pub async fn reset_failed_filings(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE file_list SET unmapped_reason = NULL WHERE mapped = FALSE AND unmapped_reason IS NOT NULL",
    )
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

/// Counts XML filings ready for ingestion that have no active failure reason
pub async fn count_pending_xml_filings(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM file_list WHERE file_type = 'XML' AND mapped = FALSE AND unmapped_reason IS NULL",
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Updates ingestion failure state for a file
pub async fn record_ingestion_failure(pool: &PgPool, file_id: Uuid, error_reason: &str) {
    let _ =
        sqlx::query("UPDATE file_list SET mapped = FALSE, unmapped_reason = $1 WHERE file_id = $2")
            .bind(error_reason)
            .bind(file_id)
            .execute(pool)
            .await;
}

/// Marks a file as successfully mapped and clears failure reasons
pub async fn mark_file_mapped(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    file_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE file_list SET mapped = TRUE, unmapped_reason = NULL WHERE file_id = $1")
        .bind(file_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}
