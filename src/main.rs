// src/main.rs

use cielo_mio::system::{AppConfig, check_service_health, start_container};
use cielo_mio::ui::BrsrApp;
use cielo_mio::ui::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let manage_db = config.manage_db_container;

    // 1. Initialize Shared Application State immediately so the window opens without blocking
    let mut app_state = AppState::new(None, config);

    // 2. Start PostgreSQL container (if managed) and connect asynchronously in the background
    if manage_db {
        cielo_mio::ui::state::trigger_db_start_and_connect(&mut app_state);
    } else {
        cielo_mio::ui::state::trigger_db_reconnect(&mut app_state);
    }

    // 3. Manage Ollama container in background
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

    // 4. Configure Native GUI Window
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
        "cielo_mio",
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
