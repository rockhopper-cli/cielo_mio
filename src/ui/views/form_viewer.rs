// src/ui/views/form_viewer.rs

pub mod components;
pub mod formatter;
pub mod nav_tree;
pub mod profile;
pub mod question_cards;
pub mod report_view;
pub mod section_b_matrix;
pub mod tables;
pub mod visibility;

use crate::ui::shell::ai_drawer::render_ai_copilot_drawer;
use crate::ui::state::{AppState, CompanyFilingSummary, FormViewerEvent};
use eframe::egui::{self, Color32, ComboBox, Layout, RichText, TextEdit, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading("📄 Official BRSR Report Explorer");
    ui.add_space(4.0);

    let pool = match state.pool.clone() {
        Some(p) => p,
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("⚠ PostgreSQL Disconnected")
                        .size(18.0)
                        .color(Color32::from_rgb(230, 90, 90)),
                );
                ui.label("Connect your database container to inspect reports.");
                ui.add_space(8.0);
                if ui
                    .button(
                        RichText::new("⚡ Attempt Reconnect")
                            .color(Color32::from_rgb(100, 180, 240)),
                    )
                    .clicked()
                {
                    crate::ui::state::trigger_db_reconnect(state);
                }
            });
            return;
        }
    };

    let company_label = if state.form_viewer.selected_cin.is_empty() {
        "None selected".to_string()
    } else {
        state.form_viewer.selected_cin.clone()
    };
    let copilot_hint = format!("Filing Explorer | Entity CIN: {}", company_label);
    render_ai_copilot_drawer(ui, state, &copilot_hint);

    // 1. Load statutory Form Template from form_master
    if !state.form_viewer.template_loaded {
        state.form_viewer.template_loaded = true;
        let pool_cloned = pool.clone();
        let tx = state.form_tx.clone();

        tokio::spawn(async move {
            match crate::storage::repository::taxonomy::fetch_form_template(&pool_cloned).await {
                Ok(template) => {
                    let _ = tx.send(FormViewerEvent::TemplateLoaded(template)).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(FormViewerEvent::Error(format!(
                            "Failed to load template: {}",
                            e
                        )))
                        .await;
                }
            }
        });
    }

    // 2. Load Ingested Filings dropdown list
    if !state.form_viewer.filings_loaded {
        state.form_viewer.filings_loaded = true;
        let pool_cloned = pool.clone();
        let tx = state.form_tx.clone();

        tokio::spawn(async move {
            match crate::storage::repository::entities::fetch_company_filings(&pool_cloned).await {
                Ok(items) => {
                    let _ = tx.send(FormViewerEvent::FilingsLoaded(items)).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(FormViewerEvent::Error(format!(
                            "Failed to load company filings: {}",
                            e
                        )))
                        .await;
                }
            }
        });
    }

    // 3. Fetch Facts when Entity Selection changes
    if let Some(selected_id) = state.form_viewer.selected_entity_id
        && state.form_viewer.cached_entity_id != Some(selected_id)
        && !state.form_viewer.is_loading_facts
    {
        state.form_viewer.is_loading_facts = true;
        let pool_cloned = pool.clone();
        let tx = state.form_tx.clone();

        tokio::spawn(async move {
            match crate::storage::repository::facts::fetch_facts_by_entity(
                &pool_cloned,
                selected_id,
            )
            .await
            {
                Ok(facts) => {
                    let _ = tx
                        .send(FormViewerEvent::FactsLoaded {
                            entity_id: selected_id,
                            facts,
                        })
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(FormViewerEvent::Error(format!(
                            "Failed to load facts: {}",
                            e
                        )))
                        .await;
                }
            }
        });
    }

    render_header_toolbar(ui, state);

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Responsive split layout with explicit separator/scrollbar margin budget
    let total_avail_w = ui.available_width();
    let total_avail_h = ui.available_height().max(240.0);
    let left_col_width = (total_avail_w * 0.27).clamp(210.0, 285.0);
    let right_width = (total_avail_w - left_col_width - 20.0).max(260.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(left_col_width, total_avail_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_max_width(left_col_width);
                nav_tree::render(ui, state, total_avail_h);
            },
        );

        ui.separator();

        ui.allocate_ui_with_layout(
            Vec2::new(right_width, total_avail_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_max_width(right_width);
                report_view::render(ui, state, total_avail_h);
            },
        );
    });
}

fn render_header_toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            let unique_companies = &state.form_viewer.unique_companies;

            ui.label(RichText::new("Company:").strong());
            let current_company_label = if let Some(selected) = unique_companies
                .iter()
                .find(|(cin, _)| cin == &state.form_viewer.selected_cin)
            {
                format!("{} ({})", selected.1, selected.0)
            } else {
                "Select a Company...".to_string()
            };

            ComboBox::from_id_salt("company_selector")
                .width(260.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .selected_text(RichText::new(current_company_label).strong())
                .show_ui(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        ui.add(
                            TextEdit::singleline(&mut state.form_viewer.company_search)
                                .hint_text("Type to jump / search 2,000+ companies...")
                                .desired_width(220.0),
                        );
                        if !state.form_viewer.company_search.is_empty()
                            && ui.small_button("✖").clicked()
                        {
                            state.form_viewer.company_search.clear();
                        }
                    });
                    ui.separator();

                    let query_lower = state.form_viewer.company_search.trim().to_lowercase();
                    let filtered_companies: Vec<&(String, String)> = unique_companies
                        .iter()
                        .filter(|(cin, name)| {
                            query_lower.is_empty()
                                || name.to_lowercase().contains(&query_lower)
                                || cin.to_lowercase().contains(&query_lower)
                        })
                        .take(80)
                        .collect();

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (cin, name) in filtered_companies {
                                let label = format!("{} ({})", name, cin);
                                if ui
                                    .selectable_value(
                                        &mut state.form_viewer.selected_cin,
                                        cin.clone(),
                                        label,
                                    )
                                    .clicked()
                                {
                                    if let Some(first_filing) = state
                                        .form_viewer
                                        .available_filings
                                        .iter()
                                        .find(|f| &f.corporate_id_number == cin)
                                    {
                                        state.form_viewer.selected_entity_id =
                                            Some(first_filing.entity_id);
                                    }
                                    state.form_viewer.company_search.clear();
                                    ui.close();
                                }
                            }
                        });
                });
            ui.add_space(8.0);

            ui.label(RichText::new("Financial Year:").strong());
            let available_years: Vec<&CompanyFilingSummary> = state
                .form_viewer
                .available_filings
                .iter()
                .filter(|f| f.corporate_id_number == state.form_viewer.selected_cin)
                .collect();

            let current_fy_label = if let Some(ent_id) = state.form_viewer.selected_entity_id {
                available_years
                    .iter()
                    .find(|f| f.entity_id == ent_id)
                    .map(|f| {
                        format!(
                            "{} → {}",
                            f.fy_start.map(|d| d.to_string()).unwrap_or_default(),
                            f.fy_end.map(|d| d.to_string()).unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|| "Select FY...".to_string())
            } else {
                "Select FY...".to_string()
            };

            ComboBox::from_id_salt("fy_selector")
                .width(175.0)
                .selected_text(current_fy_label)
                .show_ui(ui, |ui| {
                    for filing in available_years {
                        let label = format!(
                            "{} → {}",
                            filing.fy_start.map(|d| d.to_string()).unwrap_or_default(),
                            filing.fy_end.map(|d| d.to_string()).unwrap_or_default()
                        );
                        ui.selectable_value(
                            &mut state.form_viewer.selected_entity_id,
                            Some(filing.entity_id),
                            label,
                        );
                    }
                });

            ui.add_space(8.0);

            ui.label(RichText::new("🔍").strong());
            ui.add(
                TextEdit::singleline(&mut state.form_viewer.search_query)
                    .hint_text("Search question or response...")
                    .desired_width(175.0),
            );

            if !state.form_viewer.search_query.is_empty() && ui.small_button("✖").clicked() {
                state.form_viewer.search_query.clear();
            }

            if ui
                .small_button("🔄 Reload")
                .on_hover_text("Refresh template and company filings")
                .clicked()
            {
                state.form_viewer.filings_loaded = false;
                state.form_viewer.template_loaded = false;
                state.form_viewer.cached_entity_id = None;
            }
        });
    });
}
