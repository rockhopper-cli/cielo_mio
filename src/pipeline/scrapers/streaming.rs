// src/pipeline/scrapers/streaming.rs

use futures_util::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Download was cancelled by user")]
    Cancelled,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error status: {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Unexpected HTML/error payload instead of filing ({0})")]
    UnexpectedHtml(String),
}

/// Streams a file from `url` to `dest_path` atomically using a `.part` buffer
pub async fn download_file_atomic(
    client: &Client,
    url: &str,
    dest_path: &Path,
    cancel_token: &CancellationToken,
) -> Result<PathBuf, DownloadError> {
    if dest_path.exists()
        && let Ok(metadata) = fs::metadata(dest_path).await
        && metadata.len() > 0
    {
        return Ok(dest_path.to_path_buf());
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let ext = dest_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut part_path = dest_path.to_path_buf();
    if ext.is_empty() {
        part_path.set_extension("part");
    } else {
        part_path.set_extension(format!("{}.part", ext));
    }

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus(response.status()));
    }

    if let Some(ct) = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        let ct_low = ct.to_ascii_lowercase();
        if ct_low.contains("text/html") && matches!(ext.as_str(), "xml" | "pdf") {
            return Err(DownloadError::UnexpectedHtml(ct.to_string()));
        }
    }

    let mut file = File::create(&part_path).await?;
    let mut stream = response.bytes_stream();
    let mut first_bytes_checked = false;

    while let Some(chunk_result) = stream.next().await {
        if cancel_token.is_cancelled() {
            drop(file);
            let _ = fs::remove_file(&part_path).await;
            return Err(DownloadError::Cancelled);
        }

        match chunk_result {
            Ok(chunk) => {
                if !first_bytes_checked && !chunk.is_empty() {
                    first_bytes_checked = true;
                    let head = String::from_utf8_lossy(&chunk[..chunk.len().min(128)])
                        .trim_start()
                        .to_ascii_lowercase();
                    if (head.starts_with("<!doctype html") || head.starts_with("<html"))
                        && matches!(ext.as_str(), "xml" | "pdf")
                    {
                        drop(file);
                        let _ = fs::remove_file(&part_path).await;
                        return Err(DownloadError::UnexpectedHtml("HTML response body".into()));
                    }
                }

                if let Err(e) = file.write_all(&chunk).await {
                    drop(file);
                    let _ = fs::remove_file(&part_path).await;
                    return Err(DownloadError::Io(e));
                }
            }
            Err(e) => {
                drop(file);
                let _ = fs::remove_file(&part_path).await;
                return Err(DownloadError::Network(e));
            }
        }
    }

    file.flush().await?;
    drop(file);

    fs::rename(&part_path, dest_path).await?;
    Ok(dest_path.to_path_buf())
}
