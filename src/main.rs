// src/main.rs

mod ai;
mod app;
mod config;
mod container;
mod downloader;
mod parser;
mod security;

use app::BrsrApp;
use app::state::AppState;
use config::AppConfig;
use security::{KEY_DB_PASSWORD, SecretManager};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load persistent TOML configuration
    let config = AppConfig::load();

    // 2. If configured, auto-start PostgreSQL container
    if config.manage_db_container {
        println!(
            "Starting PostgreSQL container '{}'...",
            config.db_container_name
        );
        let _ = container::start_container(&config.db_container_name);
    }

    // 3. Retrieve DB password & connect
    let password = SecretManager::get_secret(KEY_DB_PASSWORD, "DB_PASSWORD").unwrap_or_default();

    let pool = downloader::db::connect_to_database(
        &config.db_host,
        config.db_port,
        &config.db_user,
        &password,
        &config.db_name,
        Duration::from_secs(3),
    )
    .await
    .ok();

    if pool.is_some() {
        println!(
            "✓ Connected to PostgreSQL database at {}:{}.",
            config.db_host, config.db_port
        );
    } else {
        println!("⚠ PostgreSQL offline or unreachable. Launching in offline mode...");
    }

    // 4. Initialize Shared Application State
    let app_state = AppState::new(pool, config);

    // Initial non-blocking check of DB container status
    app::state::trigger_db_container_check(&app_state);

    // 5. If Ollama container management is enabled, auto-start and check health
    if app_state.config.manage_ollama_container {
        let c_name = app_state.config.ollama_container_name.clone();
        let _ = container::start_container(&c_name);
    }

    let c_name = app_state.config.ollama_container_name.clone();
    let url = app_state.config.ollama_url.clone();
    let tx = app_state.container_tx.clone();
    tokio::spawn(async move {
        let status = container::check_service_health(&c_name, &url).await;
        let _ = tx.send(status).await;
    });

    // 6. Configure Native GUI Window
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_inner_size([1280.0, 800.0])
            .with_title("Cielo Mio - BRSR Extractor & ESG Analytics")
            .with_active(true),
        ..Default::default()
    };

    println!("Launching Cielo Mio Desktop Application...");

    // 7. Launch Application
    eframe::run_native(
        "BRSR Extractor",
        native_options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(1.3);
            load_custom_font(&cc.egui_ctx);
            Ok(Box::new(BrsrApp::new(cc, app_state)))
        }),
    )?;

    Ok(())
}

fn load_custom_font(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();
    fonts.font_data.insert(
        "my_system_font".to_owned(),
        std::sync::Arc::new(eframe::egui::FontData::from_static(include_bytes!(
            "../assets/InclusiveSans-VariableFont_wght.ttf"
        ))),
    );
    fonts
        .families
        .entry(eframe::egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_system_font".to_owned());
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        use eframe::egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(20.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
            (TextStyle::Small, FontId::proportional(12.0)),
        ]
        .into();
    });
}
