// src/storage/pool.rs

use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use std::time::Duration;

/// Connects to PostgreSQL with configurable connection pooling and timeout
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
