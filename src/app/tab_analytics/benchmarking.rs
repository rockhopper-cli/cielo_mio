// src/app/tab_analytics/benchmarking.rs

use crate::app::state::{ActiveTab, AppState, CompanyEsgMetrics, EsgPillar};
use eframe::egui::{self, Color32, Margin, RichText, ScrollArea, Stroke, TextEdit, Ui};

pub fn calculate_sector_median<F>(state: &AppState, sector: &str, extractor: &F) -> f64
where
    F: Fn(&CompanyEsgMetrics) -> Option<f64>,
{
    use std::collections::HashMap;
    let mut latest_by_cin: HashMap<&str, &CompanyEsgMetrics> = HashMap::new();

    for m in state
        .analytics
        .metrics
        .iter()
        .filter(|m| m.industry_sector == sector)
    {
        latest_by_cin
            .entry(&m.corporate_id_number)
            .and_modify(|existing| {
                if m.fy_start > existing.fy_start {
                    *existing = m;
                }
            })
            .or_insert(m);
    }

    let mut values: Vec<f64> = latest_by_cin
        .values()
        .filter_map(|m| extractor(m))
        .collect();
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

pub fn render_benchmarking_table(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!(
                "Cross-Company {:?} Benchmarking Matrix:",
                state.analytics.selected_pillar
            ))
            .strong(),
        );
        ui.add_space(8.0);
        ui.add(
            TextEdit::singleline(&mut state.analytics.search_query)
                .hint_text("Filter company or CIN...")
                .desired_width(200.0),
        );
        if !state.analytics.search_query.is_empty() && ui.small_button("✖").clicked() {
            state.analytics.search_query.clear();
        }
    });

    ui.add_space(4.0);

    let query_lower = state.analytics.search_query.trim().to_lowercase();
    let filtered: Vec<&CompanyEsgMetrics> = state
        .analytics
        .metrics
        .iter()
        .filter(|m| {
            let matches_query = query_lower.is_empty()
                || m.company_name.to_lowercase().contains(&query_lower)
                || m.corporate_id_number.to_lowercase().contains(&query_lower);
            let matches_sector = state.analytics.selected_sector == "All Sectors"
                || m.industry_sector == state.analytics.selected_sector;
            matches_query && matches_sector
        })
        .collect();

    let header_color = Color32::from_rgb(100, 180, 240);

    egui::Frame::canvas(ui.style())
        .fill(Color32::from_rgb(18, 20, 26))
        .stroke(Stroke::new(1.0, Color32::from_rgb(42, 48, 60)))
        .inner_margin(Margin::symmetric(6, 6))
        .show(ui, |ui| {
            ScrollArea::both()
                .max_height(240.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("analytics_grid")
                        .striped(true)
                        .min_col_width(85.0)
                        .spacing([14.0, 7.0])
                        .show(ui, |ui| {
                            // Common Headers
                            ui.label(RichText::new("Company").strong().color(header_color));
                            ui.label(RichText::new("Sector").strong().color(header_color));
                            ui.label(RichText::new("FY").strong().color(header_color));

                            // Dynamic Headers based on Pillar
                            match state.analytics.selected_pillar {
                                EsgPillar::Environmental => {
                                    ui.label(
                                        RichText::new("Scope 1 (t)").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Scope 2 (t)").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Scope 3 (t)").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Water (KL)").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Energy Int.").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Waste Int.").strong().color(header_color),
                                    );
                                }
                                EsgPillar::Social => {
                                    ui.label(
                                        RichText::new("Board Women %").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Female Wages %")
                                            .strong()
                                            .color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Turnover %").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Injuries").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Fatalities").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Workforce %").strong().color(header_color),
                                    );
                                }
                                EsgPillar::Governance => {
                                    ui.label(
                                        RichText::new("POSH Upheld").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Total POSH").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Complaints Filed")
                                            .strong()
                                            .color(header_color),
                                    );
                                    ui.label(RichText::new("Pending").strong().color(header_color));
                                    ui.label(
                                        RichText::new("Disciplinary").strong().color(header_color),
                                    );
                                    ui.label(
                                        RichText::new("Well-Being").strong().color(header_color),
                                    );
                                }
                            }

                            ui.label(RichText::new("Action").strong().color(header_color));
                            ui.end_row();

                            // Rows
                            for m in filtered {
                                ui.label(RichText::new(&m.company_name).strong());
                                ui.label(
                                    RichText::new(&m.industry_sector)
                                        .small()
                                        .color(Color32::from_rgb(160, 170, 190)),
                                );
                                ui.label(RichText::new(&m.fy_label).small());

                                match state.analytics.selected_pillar {
                                    EsgPillar::Environmental => {
                                        ui.label(fmt_opt(m.scope_1_tco2e));
                                        ui.label(fmt_opt(m.scope_2_tco2e));
                                        ui.label(fmt_opt(m.scope_3_tco2e));
                                        ui.label(fmt_opt(m.water_withdrawal_kl));
                                        ui.label(fmt_opt(m.energy_intensity));
                                        ui.label(fmt_opt(m.waste_intensity));
                                    }
                                    EsgPillar::Social => {
                                        ui.label(fmt_opt_pct(m.female_board_directors_pct));
                                        ui.label(fmt_opt_pct(m.gross_wages_to_female_pct));
                                        ui.label(fmt_opt_pct(m.employee_turnover_pct));
                                        ui.label(fmt_opt(m.work_related_injuries));
                                        ui.label(fmt_opt(m.fatalities));
                                        ui.label(fmt_opt_pct(m.female_workforce_pct));
                                    }
                                    EsgPillar::Governance => {
                                        ui.label(fmt_opt(m.posh_complaints_upheld));
                                        ui.label(fmt_opt(m.total_posh_complaints));
                                        ui.label(fmt_opt(m.complaints_filed));
                                        ui.label(fmt_opt(m.pending_complaints));
                                        ui.label(fmt_opt(m.disciplinary_actions));
                                        ui.label(fmt_opt(m.cost_on_wellbeing_pct));
                                    }
                                }

                                if ui.small_button("📄 View Form").clicked() {
                                    state.form_viewer.selected_cin = m.corporate_id_number.clone();
                                    state.form_viewer.selected_entity_id = Some(m.entity_id);
                                    state.active_tab = ActiveTab::FormViewer;
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}

fn fmt_opt(val: Option<f64>) -> String {
    val.map(|v| format!("{:.2}", v))
        .unwrap_or_else(|| "—".to_string())
}

fn fmt_opt_pct(val: Option<f64>) -> String {
    val.map(|v| format!("{:.1}%", v))
        .unwrap_or_else(|| "—".to_string())
}
