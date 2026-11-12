// src/ui/views/geo.rs

use crate::ui::state::{ActiveTab, AppState, GeoEntityMarker};
use eframe::egui::{self, Color32, ComboBox, Frame, Margin, RichText, Stroke, TextEdit, Ui};
use std::sync::atomic::Ordering;
use uuid::Uuid;
use walkers::{
    HttpTiles, Map, MapMemory, Plugin, Position, Projector, lat_lon, sources::OpenStreetMap,
};

struct GeoEntityPlugin {
    markers: Vec<GeoEntityMarker>,
    selected_entity_id: Option<Uuid>,
}

impl Plugin for GeoEntityPlugin {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        _response: &egui::Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        let painter = ui.painter();

        for m in &self.markers {
            let pos: Position = lat_lon(m.latitude, m.longitude);
            let screen_pos = projector.project(pos).to_pos2();

            if !ui.clip_rect().contains(screen_pos) {
                continue;
            }

            let is_selected = self.selected_entity_id == Some(m.entity_id);
            let base_radius = if is_selected { 9.0 } else { 6.5 };

            let hit_rect = egui::Rect::from_center_size(
                screen_pos,
                egui::vec2(base_radius * 2.6, base_radius * 2.6),
            );
            let pin_resp = ui.interact(
                hit_rect,
                egui::Id::unique(m.entity_id),
                egui::Sense::click(),
            );
            let is_hovered = pin_resp.hovered();

            if pin_resp.clicked() {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::unique("clicked_geo_pin"), m.entity_id);
                });
            }

            let (fill_color, stroke_color, stroke_width) = if is_selected {
                (Color32::from_rgb(255, 185, 40), Color32::WHITE, 2.0)
            } else if is_hovered {
                (Color32::from_rgb(240, 80, 80), Color32::WHITE, 2.0)
            } else {
                (
                    Color32::from_rgb(46, 180, 110),
                    Color32::from_rgb(20, 25, 35),
                    1.5,
                )
            };

            if is_selected {
                painter.circle_filled(
                    screen_pos,
                    base_radius + 5.0,
                    Color32::from_rgba_premultiplied(255, 185, 40, 60),
                );
            }

            painter.circle_filled(screen_pos, base_radius, fill_color);
            painter.circle_stroke(
                screen_pos,
                base_radius,
                Stroke::new(stroke_width, stroke_color),
            );
            painter.circle_filled(screen_pos, 2.0, Color32::WHITE);

            if is_hovered || is_selected {
                painter.text(
                    screen_pos + egui::vec2(base_radius + 6.0, -1.0),
                    egui::Align2::LEFT_CENTER,
                    &m.company_name,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }

            pin_resp.on_hover_ui(|ui| {
                ui.heading(&m.company_name);
                ui.label(
                    RichText::new(&m.corporate_id_number)
                        .small()
                        .color(Color32::from_rgb(160, 175, 195)),
                );
                ui.separator();
                ui.label(format!("🏢 Sector: {}", m.industry_sector));
                ui.label(format!("📍 Location: {}, {}", m.city_name, m.state_name));
                ui.label(format!(
                    "🌐 Coordinates: {:.4}°N, {:.4}°E",
                    m.latitude, m.longitude
                ));
                if let Some(s1) = m.scope_1_tco2e {
                    ui.label(format!("🌱 Scope 1: {:.1} tCO2e", s1));
                }
                if let Some(w) = m.water_withdrawal_kl {
                    ui.label(format!("💧 Water: {:.0} KL", w));
                }
            });
        }
    }
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("🗺 Geospatial ESG Distribution & Regional Clusters");
    ui.add_space(4.0);

    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            ui.label("Database disconnected. Connect database to view geographic distribution.");
            return;
        }
    };

    if state.geo_state.markers.is_empty() && !state.geo_state.is_loading {
        state.geo_state.is_loading = true;
        let p_cloned = pool.clone();
        let tx = state.geo_tx.clone();
        tokio::spawn(async move {
            let res = crate::storage::repository::entities::fetch_geo_entities(&p_cloned)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(res).await;
        });
    }

    if let Some(clicked_id) = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<Uuid>(egui::Id::unique("clicked_geo_pin")))
    {
        state.geo_state.selected_entity_id = Some(clicked_id);
    }

    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Sector:").strong());
            ComboBox::from_id_salt("geo_sector_combo")
                .width(180.0)
                .selected_text(&state.geo_state.selected_sector)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.geo_state.selected_sector,
                        "All Sectors".to_string(),
                        "All Sectors",
                    );
                    let mut sectors: Vec<String> = state
                        .geo_state
                        .markers
                        .iter()
                        .map(|m| m.industry_sector.clone())
                        .collect();
                    sectors.sort();
                    sectors.dedup();
                    for s in sectors {
                        ui.selectable_value(&mut state.geo_state.selected_sector, s.clone(), &s);
                    }
                });

            ui.separator();

            ui.add(
                TextEdit::singleline(&mut state.geo_state.search_query)
                    .hint_text("Search company, city, state...")
                    .desired_width(180.0),
            );

            ui.separator();

            let is_geocoding = state.geo_state.is_geocoding_running.load(Ordering::Relaxed);
            if !is_geocoding {
                if ui
                    .button(
                        RichText::new("📍 Resolve Pending Coordinates")
                            .small()
                            .color(Color32::from_rgb(110, 200, 255)),
                    )
                    .on_hover_text(
                        "Instantly geocode unmapped entity addresses using the local PIN code master",
                    )
                    .clicked()
                {
                    let flag = state.geo_state.is_geocoding_running.clone();
                    let pool_clone = pool.clone();
                    let geo_tx = state.geo_tx.clone();
                    flag.store(true, Ordering::Relaxed);
                    tokio::spawn(async move {
                        let geocoder =
                            crate::pipeline::geocoder::GeocoderService::new(pool_clone.clone());
                        let _ = geocoder.process_pending_batch(0).await;
                        let res =
                            crate::storage::repository::entities::fetch_geo_entities(&pool_clone)
                                .await
                                .map_err(|e| e.to_string());
                        let _ = geo_tx.send(res).await;
                        flag.store(false, Ordering::Relaxed);
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Resolving coordinates in PostgreSQL...")
                            .small()
                            .color(Color32::YELLOW),
                    );
                });
            }
        });
    });

    ui.add_space(6.0);

    let q_lower = state.geo_state.search_query.trim().to_lowercase();
    let filtered_markers: Vec<GeoEntityMarker> = state
        .geo_state
        .markers
        .iter()
        .filter(|m| {
            let sector_match = state.geo_state.selected_sector == "All Sectors"
                || m.industry_sector == state.geo_state.selected_sector;
            let query_match = q_lower.is_empty()
                || m.company_name.to_lowercase().contains(&q_lower)
                || m.city_name.to_lowercase().contains(&q_lower)
                || m.state_name.to_lowercase().contains(&q_lower);
            sector_match && query_match
        })
        .cloned()
        .collect();

    let body_h = (ui.available_height() - 48.0).clamp(320.0, 520.0);

    ui.columns(2, |cols| {
        cols[0].group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Interactive Map ({} plotted)",
                        filtered_markers.len()
                    ))
                    .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("➕ Zoom In").clicked() {
                        let _ = state.geo_state.map_memory.zoom_in();
                    }
                    if ui.small_button("➖ Zoom Out").clicked() {
                        let _ = state.geo_state.map_memory.zoom_out();
                    }
                    if ui.small_button("🇮🇳 All India").clicked() {
                        state.geo_state.map_memory.center_at(lat_lon(22.0, 79.0));
                        let _ = state.geo_state.map_memory.set_zoom(5.0);
                        state.geo_state.selected_entity_id = None;
                    }
                });
            });

            ui.add_space(4.0);

            let default_center = lat_lon(22.0, 79.0);
            let selected_id = state.geo_state.selected_entity_id;

            let tiles = state
                .geo_state
                .tiles
                .get_or_insert_with(|| HttpTiles::new(OpenStreetMap, ui.ctx().clone()));

            let map = Map::new(Some(tiles), &mut state.geo_state.map_memory, default_center)
                .with_plugin(GeoEntityPlugin {
                    markers: filtered_markers.clone(),
                    selected_entity_id: selected_id,
                });

            Frame::canvas(ui.style())
                .stroke(Stroke::new(1.0, Color32::from_rgb(45, 52, 65)))
                .corner_radius(4)
                .show(ui, |ui| {
                    ui.set_height(body_h);
                    ui.add(map);
                });
        });

        cols[1].group(|ui| {
            ui.label(RichText::new("Top Industrial & Corporate Hubs").strong());
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(body_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for m in filtered_markers.iter().take(50) {
                        let is_selected = state.geo_state.selected_entity_id == Some(m.entity_id);
                        let frame_bg = if is_selected {
                            Color32::from_rgb(34, 42, 58)
                        } else {
                            Color32::from_rgb(24, 28, 38)
                        };
                        let stroke_color = if is_selected {
                            Color32::from_rgb(255, 185, 40)
                        } else {
                            Color32::from_rgb(45, 52, 65)
                        };

                        Frame::NONE
                            .fill(frame_bg)
                            .stroke(Stroke::new(1.0, stroke_color))
                            .corner_radius(4)
                            .inner_margin(Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(&m.company_name).strong());
                                    ui.label(
                                        RichText::new(format!("• {}", m.city_name))
                                            .small()
                                            .color(Color32::from_rgb(110, 190, 255)),
                                    );
                                });

                                ui.label(
                                    RichText::new(format!("• Sector: {}", m.industry_sector))
                                        .small()
                                        .color(Color32::GRAY),
                                );

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!("• State: {}", m.state_name))
                                            .small()
                                            .color(Color32::from_rgb(180, 190, 205)),
                                    );

                                    if ui.small_button("📍 Focus").clicked() {
                                        state
                                            .geo_state
                                            .map_memory
                                            .center_at(lat_lon(m.latitude, m.longitude));
                                        let _ = state.geo_state.map_memory.set_zoom(11.0);
                                        state.geo_state.selected_entity_id = Some(m.entity_id);
                                    }

                                    if ui.small_button("📄 View Filing").clicked() {
                                        state.form_viewer.selected_cin =
                                            m.corporate_id_number.clone();
                                        state.form_viewer.selected_entity_id = Some(m.entity_id);
                                        state.active_tab = ActiveTab::FormViewer;
                                    }
                                });
                            });

                        ui.add_space(4.0);
                    }
                });
        });
    });
}
