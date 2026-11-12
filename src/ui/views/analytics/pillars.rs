// src/ui/views/analytics/pillars.rs

use super::benchmarking::calculate_sector_median;
use super::charts;
use crate::ui::state::{
    AppState, CompanyEsgMetrics, EnvironmentalMetric, EsgPillar, GovernanceMetric, SocialMetric,
};
use eframe::egui::{self, Color32, ComboBox, Margin, RichText, Stroke, TextEdit, Ui, Vec2};
use egui_plot::Bar;

type MetricExtractor = Box<dyn Fn(&CompanyEsgMetrics) -> Option<f64>>;

pub fn render_active_metric_view(ui: &mut Ui, state: &mut AppState) {
    let unique_companies = &state.analytics.unique_companies;

    let current_name = unique_companies
        .iter()
        .find(|(cin, _)| cin == &state.analytics.selected_cin)
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| {
            if state.analytics.selected_cin.is_empty() {
                "Select a Company...".to_string()
            } else {
                state.analytics.selected_cin.clone()
            }
        });

    let company_records: Vec<&CompanyEsgMetrics> = state
        .analytics
        .metrics
        .iter()
        .filter(|m| m.corporate_id_number == state.analytics.selected_cin)
        .collect();

    let latest_opt = company_records.last().copied();

    egui::Frame::NONE
        .fill(Color32::from_rgb(22, 26, 35))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 55, 75)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);

                ui.label(RichText::new("Entity:").strong());

                ComboBox::from_id_salt("pillar_company_selector")
                    .width(260.0)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .selected_text(RichText::new(&current_name).strong())
                    .show_ui(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            ui.add(
                                TextEdit::singleline(&mut state.analytics.company_search)
                                    .hint_text("Type to jump / search...")
                                    .desired_width(210.0),
                            );
                            if !state.analytics.company_search.is_empty()
                                && ui.small_button("✖").clicked()
                            {
                                state.analytics.company_search.clear();
                            }
                        });
                        ui.separator();

                        let query_lower = state.analytics.company_search.trim().to_lowercase();
                        let filtered: Vec<&(String, String)> = unique_companies
                            .iter()
                            .filter(|(cin, name)| {
                                query_lower.is_empty()
                                    || name.to_lowercase().contains(&query_lower)
                                    || cin.to_lowercase().contains(&query_lower)
                            })
                            .take(100)
                            .collect();

                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                for (cin, name) in filtered {
                                    let label = format!("{} ({})", name, cin);
                                    if ui
                                        .selectable_value(
                                            &mut state.analytics.selected_cin,
                                            cin.clone(),
                                            label,
                                        )
                                        .clicked()
                                    {
                                        state.analytics.company_search.clear();
                                        ui.close();
                                    }
                                }
                            });
                    });

                if let Some(latest) = latest_opt {
                    ui.label(
                        RichText::new(format!("• Sector: {}", latest.industry_sector))
                            .color(Color32::from_rgb(130, 200, 155))
                            .small(),
                    );

                    if !latest.industry_division.is_empty() && latest.industry_division != "General"
                    {
                        ui.label(
                            RichText::new(format!("• Division: {}", latest.industry_division))
                                .color(Color32::from_rgb(160, 170, 190))
                                .small(),
                        );
                    }

                    if let Some(cap) = latest.paid_up_capital_cr {
                        ui.label(
                            RichText::new(format!("• Paid-Up: ₹ {:.2} Cr", cap))
                                .color(Color32::from_rgb(230, 190, 80))
                                .small(),
                        );
                    }

                    let action_btn = ui.add(
                        egui::Button::new(
                            RichText::new("📄 View Form Filing ➔")
                                .color(Color32::from_rgb(110, 190, 255))
                                .strong(),
                        )
                        .fill(Color32::from_rgb(32, 42, 60)),
                    );

                    if action_btn.clicked() {
                        state.form_viewer.selected_cin = state.analytics.selected_cin.clone();
                        state.form_viewer.selected_entity_id = Some(latest.entity_id);
                        state.active_tab = crate::ui::state::ActiveTab::FormViewer;
                    }
                }
            });
        });

    ui.add_space(8.0);

    if company_records.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new("ℹ Select a company from the dropdown above to view ESG metrics.")
                    .color(Color32::from_rgb(160, 170, 185)),
            );
            ui.add_space(30.0);
        });
        return;
    }

    match state.analytics.selected_pillar {
        EsgPillar::Environmental => render_environmental_metric(ui, &company_records, state),
        EsgPillar::Social => render_social_metric(ui, &company_records, state),
        EsgPillar::Governance => render_governance_metric(ui, &company_records, state),
    }
}

fn render_environmental_metric(ui: &mut Ui, records: &[&CompanyEsgMetrics], state: &AppState) {
    let met = state.analytics.selected_env_metric;
    let extractor: MetricExtractor = match met {
        EnvironmentalMetric::Scope1Emissions => Box::new(|m| m.scope_1_tco2e),
        EnvironmentalMetric::Scope2Emissions => Box::new(|m| m.scope_2_tco2e),
        EnvironmentalMetric::Scope3Emissions => Box::new(|m| m.scope_3_tco2e),
        EnvironmentalMetric::RenewableEnergy => Box::new(|m| m.renewable_energy_pct),
        EnvironmentalMetric::WaterWithdrawal => Box::new(|m| m.water_withdrawal_kl),
        EnvironmentalMetric::EnergyIntensity => Box::new(|m| m.energy_intensity),
        EnvironmentalMetric::WasteIntensity => Box::new(|m| m.waste_intensity),
    };

    render_metric_dashboard_layout(
        ui,
        records,
        state,
        super::get_active_metric_label(state),
        extractor,
        Color32::from_rgb(46, 125, 50),
    );
}

fn render_social_metric(ui: &mut Ui, records: &[&CompanyEsgMetrics], state: &AppState) {
    let met = state.analytics.selected_soc_metric;
    let extractor: MetricExtractor = match met {
        SocialMetric::FemaleBoardDirectors => Box::new(|m| m.female_board_directors_pct),
        SocialMetric::GrossWagesToFemale => Box::new(|m| m.gross_wages_to_female_pct),
        SocialMetric::EmployeeTurnoverRate => Box::new(|m| m.employee_turnover_pct),
        SocialMetric::WorkRelatedInjuries => Box::new(|m| m.work_related_injuries),
        SocialMetric::Fatalities => Box::new(|m| m.fatalities),
        SocialMetric::FemaleWorkforce => Box::new(|m| m.female_workforce_pct),
    };

    render_metric_dashboard_layout(
        ui,
        records,
        state,
        super::get_active_metric_label(state),
        extractor,
        Color32::from_rgb(25, 118, 210),
    );
}

fn render_governance_metric(ui: &mut Ui, records: &[&CompanyEsgMetrics], state: &AppState) {
    let met = state.analytics.selected_gov_metric;
    let extractor: MetricExtractor = match met {
        GovernanceMetric::PoshComplaintsUpheld => Box::new(|m| m.posh_complaints_upheld),
        GovernanceMetric::TotalPoshComplaints => Box::new(|m| m.total_posh_complaints),
        GovernanceMetric::ComplaintsFiled => Box::new(|m| m.complaints_filed),
        GovernanceMetric::PendingComplaints => Box::new(|m| m.pending_complaints),
        GovernanceMetric::DisciplinaryActions => Box::new(|m| m.disciplinary_actions),
        GovernanceMetric::CostOnWellBeing => Box::new(|m| m.cost_on_wellbeing_pct),
    };

    render_metric_dashboard_layout(
        ui,
        records,
        state,
        super::get_active_metric_label(state),
        extractor,
        Color32::from_rgb(180, 130, 40),
    );
}

fn render_metric_dashboard_layout(
    ui: &mut Ui,
    records: &[&CompanyEsgMetrics],
    state: &AppState,
    metric_name: &str,
    extractor: MetricExtractor,
    theme_color: Color32,
) {
    let latest = records.last().unwrap();
    let latest_val_opt = extractor(latest);
    let sector_median = calculate_sector_median(state, &latest.industry_sector, &extractor);

    ui.columns(2, |cols| {
        cols[0].group(|ui| {
            ui.label(RichText::new(format!("{} YoY Progress:", metric_name)).strong());
            ui.add_space(4.0);

            let mut bars = Vec::new();
            for (i, r) in records.iter().enumerate() {
                if let Some(v) = extractor(r) {
                    bars.push(Bar::new(i as f64 + 0.5, v).name(&r.fy_label).width(0.45));
                }
            }

            if bars.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "ℹ Not Disclosed in Filings for '{}'",
                            latest.company_name
                        ))
                        .color(Color32::from_rgb(160, 170, 185)),
                    );
                });
                ui.add_space(40.0);
            } else {
                charts::render_single_metric_chart(
                    ui,
                    "metric_yoy_plot",
                    metric_name,
                    bars,
                    theme_color,
                    190.0,
                );
            }
        });

        cols[1].group(|ui| {
            ui.label(RichText::new(format!("{} vs Sector Median:", metric_name)).strong());
            ui.add_space(4.0);

            if let Some(c_val) = latest_val_opt {
                ui.label(format!(
                    "Company: {:.2} | Sector Median: {:.2}",
                    c_val, sector_median
                ));
                charts::render_peer_comparison(ui, "peer_comp_plot", c_val, sector_median);
            } else if sector_median > 0.0 {
                ui.label(format!(
                    "Company: Not Disclosed | Sector Median: {:.2}",
                    sector_median
                ));
                charts::render_peer_comparison(ui, "peer_comp_plot", 0.0, sector_median);
            } else {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "ℹ No disclosures reported for this metric in sector '{}'",
                            latest.industry_sector
                        ))
                        .color(Color32::from_rgb(160, 170, 185)),
                    );
                });
                ui.add_space(40.0);
            }
        });
    });
}
