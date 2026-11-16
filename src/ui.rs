// src/ui.rs

pub mod events;
pub mod shell;
pub mod state;
pub mod views;

use eframe::egui::{self, Color32, Frame, Margin};
use state::{ActiveTab, AppState};
use std::sync::atomic::Ordering;

pub struct BrsrApp {
    pub state: AppState,
}

impl BrsrApp {
    pub fn new(cc: &eframe::CreationContext<'_>, state: AppState) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(15, 17, 21);
        visuals.panel_fill = Color32::from_rgb(20, 23, 28);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 28, 35);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 33, 41);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 49, 61);
        visuals.widgets.active.bg_fill = Color32::from_rgb(52, 62, 78);
        cc.egui_ctx.set_visuals(visuals);

        Self { state }
    }
}

impl eframe::App for BrsrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        events::drain_incoming_events(&mut self.state);

        shell::nav_rail::render(ui, &mut self.state);
        shell::logs_panel::render(ui, &mut self.state);

        let central_frame = Frame::central_panel(ui.style())
            .fill(Color32::from_rgb(15, 17, 21))
            .inner_margin(Margin::symmetric(16, 12));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ui, |ui| match self.state.active_tab {
                ActiveTab::Download => views::download::render(ui, &mut self.state),
                ActiveTab::FormViewer => views::form_viewer::render(ui, &mut self.state),
                ActiveTab::Analytics => views::analytics::render(ui, &mut self.state),
                ActiveTab::GeoReports => views::geo::render(ui, &mut self.state),
                ActiveTab::Export => views::export::render(ui, &mut self.state),
                ActiveTab::Config => views::config::render(ui, &mut self.state),
                ActiveTab::About => views::about::render(ui, &mut self.state),
            });

        if self.state.is_downloading.load(Ordering::Relaxed)
            || self.state.is_ingesting.load(Ordering::Relaxed)
            || self.state.ai.is_thinking.load(Ordering::Relaxed)
            || self.state.is_reconnecting_db.load(Ordering::Relaxed)
            || self.state.form_viewer.is_loading_facts
            || self.state.analytics.is_loading
            || self.state.geo_state.is_loading
            || self.state.export_state.is_exporting.load(Ordering::Relaxed)
            || self
                .state
                .export_state
                .is_dumping_db
                .load(Ordering::Relaxed)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
    }
}

impl Drop for BrsrApp {
    fn drop(&mut self) {
        let _ = self.state.config.save();

        let stop_ollama = self
            .state
            .config
            .manage_ollama_container
            .then(|| self.state.config.ollama_container_name.clone());
        let stop_db = self
            .state
            .config
            .manage_db_container
            .then(|| self.state.config.db_container_name.clone());

        std::thread::scope(|s| {
            if let Some(ref name) = stop_ollama {
                s.spawn(|| {
                    let _ = crate::system::stop_container(name);
                });
            }
            if let Some(ref name) = stop_db {
                s.spawn(|| {
                    let _ = crate::system::stop_container(name);
                });
            }
        });
    }
}
