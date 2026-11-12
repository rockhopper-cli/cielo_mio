// src/main.rs

use cielo_mio::storage::connect_to_database;
use cielo_mio::system::{
    AppConfig, KEY_DB_PASSWORD, SecretManager, check_service_health, start_container,
};
use cielo_mio::ui::BrsrApp;
use cielo_mio::ui::state::AppState;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();

    // 1. Start PostgreSQL container if managed
    let mut started_db_container = false;
    if config.manage_db_container {
        println!(
            "Starting PostgreSQL container '{}'...",
            config.db_container_name
        );
        match start_container(&config.db_container_name) {
            Ok(msg) => {
                println!("✓ {}", msg);
                started_db_container = true;
            }
            Err(e) => eprintln!("⚠ Could not start DB container: {}", e),
        }
    }

    // 2. Retrieve DB password & connect (with readiness retries if container just started)
    let password = SecretManager::get_secret(KEY_DB_PASSWORD, "DB_PASSWORD").unwrap_or_default();
    let max_attempts = if started_db_container { 5 } else { 1 };
    let mut pool = None;

    for attempt in 1..=max_attempts {
        match connect_to_database(
            &config.db_host,
            config.db_port,
            &config.db_user,
            &password,
            &config.db_name,
            Duration::from_secs(3),
        )
        .await
        {
            Ok(p) => {
                pool = Some(p);
                break;
            }
            Err(e) => {
                if attempt < max_attempts {
                    tokio::time::sleep(Duration::from_millis(900)).await;
                } else {
                    eprintln!("⚠ PostgreSQL connection attempt failed: {}", e);
                }
            }
        }
    }

    if let Some(ref p) = pool {
        println!(
            "✓ Connected to PostgreSQL database at {}:{}.",
            config.db_host, config.db_port
        );
        let p_clone = p.clone();
        tokio::spawn(async move {
            let _ = cielo_mio::storage::repository::entities::reclassify_unclassified_entities(
                &p_clone,
            )
            .await;
        });
    } else {
        println!("⚠ PostgreSQL offline or unreachable. Launching in offline mode...");
    }

    // 3. Initialize Shared Application State
    let app_state = AppState::new(pool, config);
    cielo_mio::ui::state::trigger_db_container_check(&app_state);

    // 4. Manage Ollama container in background so GUI appears immediately
    let manage_ollama = app_state.config.manage_ollama_container;
    let c_name = app_state.config.ollama_container_name.clone();
    let url = app_state.config.ollama_url.clone();
    let tx = app_state.container_tx.clone();

    tokio::spawn(async move {
        if manage_ollama {
            let c = c_name.clone();
            let _ = tokio::task::spawn_blocking(move || start_container(&c)).await;
        }
        let status = check_service_health(&c_name, &url).await;
        let _ = tx.send(status).await;
    });

    // 5. Configure Native GUI Window
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_inner_size([1280.0, 800.0])
            .with_title("Cielo Mio - BRSR Extractor & ESG Analytics")
            .with_active(true),
        ..Default::default()
    };

    println!("Launching Cielo Mio Desktop Application...");

    eframe::run_native(
        "BRSR Extractor",
        native_options,
        Box::new(move |cc| {
            cc.egui_ctx.set_zoom_factor(1.25);
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
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(20.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(14.5));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(13.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));
    });
}
