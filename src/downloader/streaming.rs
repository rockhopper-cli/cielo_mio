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
}

/// Streams a file from `url` to `dest_path` atomically using a `.part` buffer.
/// If already present and non-empty, skips download.
pub async fn download_file_atomic(
    client: &Client,
    url: &str,
    dest_path: &Path,
    cancel_token: &CancellationToken,
) -> Result<PathBuf, DownloadError> {
    // 1. If file already exists and is non-empty, avoid redundant download
    if dest_path.exists() {
        if let Ok(metadata) = fs::metadata(dest_path).await {
            if metadata.len() > 0 {
                return Ok(dest_path.to_path_buf());
            }
        }
    }

    // 2. Ensure parent output directory exists
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // 3. Define temp file: <dest_path>.part
    let mut part_path = dest_path.to_path_buf();
    let ext = part_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    part_path.set_extension(format!("{}.part", ext));

    // 4. Request the file stream
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus(response.status()));
    }

    // 5. Stream chunks to the .part file
    let mut file = File::create(&part_path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation signal
        if cancel_token.is_cancelled() {
            drop(file);
            let _ = fs::remove_file(&part_path).await;
            return Err(DownloadError::Cancelled);
        }

        match chunk_result {
            Ok(chunk) => {
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

    // Flush and close file
    file.flush().await?;
    drop(file);

    // 6. Atomically rename .part -> final target
    fs::rename(&part_path, dest_path).await?;

    Ok(dest_path.to_path_buf())
}
