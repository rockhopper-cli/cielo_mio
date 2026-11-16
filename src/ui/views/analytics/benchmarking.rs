// src/ui/views/analytics/benchmarking.rs

use crate::storage::repository::analytics::SECTOR_BENCHMARKS;
use crate::ui::state::{
    ActiveTab, AppState, CompanyEsgMetrics, EnvironmentalMetric, EsgPillar, GovernanceMetric,
    SocialMetric,
};
use eframe::egui::{self, Color32, ComboBox, Margin, RichText, ScrollArea, Stroke, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};

pub fn calculate_sector_median<F>(state: &AppState, sector: &str, _extractor: &F) -> Option<f64>
where
    F: Fn(&CompanyEsgMetrics) -> Option<f64>,
{
    let col_name = get_active_median_column_name(state);
    SECTOR_BENCHMARKS
        .read()
        .ok()
        .and_then(|lock| lock.get(sector).and_then(|m| m.get(col_name)).copied())
}

fn get_active_median_column_name(state: &AppState) -> &'static str {
    match state.analytics.selected_pillar {
        EsgPillar::Environmental => match state.analytics.selected_env_metric {
            EnvironmentalMetric::Scope1Emissions => "median_scope_1",
            EnvironmentalMetric::Scope2Emissions => "median_scope_2",
            EnvironmentalMetric::Scope3Emissions => "median_scope_3",
            EnvironmentalMetric::RenewableEnergy => "median_renewable_energy_pct",
            EnvironmentalMetric::WaterWithdrawal => "median_water",
            EnvironmentalMetric::EnergyIntensity => "median_energy_intensity",
            EnvironmentalMetric::WasteIntensity => "median_waste_intensity",
        },
        EsgPillar::Social => match state.analytics.selected_soc_metric {
            SocialMetric::FemaleBoardDirectors => "median_female_board_pct",
            SocialMetric::GrossWagesToFemale => "median_gross_wages_pct",
            SocialMetric::EmployeeTurnoverRate => "median_turnover_pct",
            SocialMetric::WorkRelatedInjuries => "median_injuries",
            SocialMetric::Fatalities => "median_fatalities",
            SocialMetric::FemaleWorkforce => "median_female_workforce_pct",
        },
        EsgPillar::Governance => match state.analytics.selected_gov_metric {
            GovernanceMetric::PoshComplaintsUpheld => "median_posh_upheld",
            GovernanceMetric::TotalPoshComplaints => "median_total_posh",
            GovernanceMetric::ComplaintsFiled => "median_complaints_filed",
            GovernanceMetric::PendingComplaints => "median_pending_complaints",
            GovernanceMetric::DisciplinaryActions => "median_disciplinary_actions",
            GovernanceMetric::CostOnWellBeing => "median_cost_wellbeing_pct",
        },
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
                .desired_width(180.0),
        );
        if !state.analytics.search_query.is_empty() && ui.small_button("✖").clicked() {
            state.analytics.search_query.clear();
        }

        ui.add_space(8.0);
        ui.label("Sector:");
        ComboBox::from_id_salt("analytics_sector_filter")
            .width(180.0)
            .selected_text(&state.analytics.selected_sector)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.analytics.selected_sector,
                    "All Sectors".to_string(),
                    "All Sectors",
                );
                let mut sectors: Vec<String> = state
                    .analytics
                    .metrics
                    .iter()
                    .map(|m| m.industry_sector.clone())
                    .collect();
                sectors.sort();
                sectors.dedup();
                for s in sectors {
                    ui.selectable_value(&mut state.analytics.selected_sector, s.clone(), &s);
                }
            });
    });

    ui.add_space(6.0);

    let query_lower = state.analytics.search_query.trim().to_lowercase();
    let filtered: Vec<CompanyEsgMetrics> = state
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
        .cloned()
        .collect();

    let header_color = Color32::from_rgb(100, 180, 240);
    let pillar = state.analytics.selected_pillar;
    let mut clicked_view: Option<(String, uuid::Uuid)> = None;

    egui::Frame::canvas(ui.style())
        .fill(Color32::from_rgb(18, 20, 26))
        .stroke(Stroke::new(1.0, Color32::from_rgb(42, 48, 60)))
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt("analytics_matrix_hscroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let metric_col_count = match pillar {
                        EsgPillar::Environmental => 7,
                        EsgPillar::Social | EsgPillar::Governance => 6,
                    };

                    let mut builder = TableBuilder::new(ui)
                        .id_salt(("analytics_table", pillar as u8))
                        .striped(true)
                        .resizable(true)
                        .min_scrolled_height(240.0)
                        .max_scroll_height(280.0)
                        .column(Column::exact(190.0))
                        .column(Column::exact(155.0))
                        .column(Column::exact(115.0));

                    for _ in 0..metric_col_count {
                        builder = builder.column(Column::exact(95.0));
                    }
                    builder = builder.column(Column::exact(90.0));

                    builder
                        .header(26.0, |mut header| {
                            header.col(|ui| {
                                ui.label(RichText::new("Company").strong().color(header_color));
                            });
                            header.col(|ui| {
                                ui.label(RichText::new("Sector").strong().color(header_color));
                            });
                            header.col(|ui| {
                                ui.label(RichText::new("FY").strong().color(header_color));
                            });

                            let metric_headers: &[&str] = match pillar {
                                EsgPillar::Environmental => &[
                                    "Scope 1 (t)",
                                    "Scope 2 (t)",
                                    "Scope 3 (t)",
                                    "Renew %",
                                    "Water (KL)",
                                    "Energy Int.",
                                    "Waste Int.",
                                ],
                                EsgPillar::Social => &[
                                    "Board Women %",
                                    "Female Wages %",
                                    "Turnover %",
                                    "Injuries",
                                    "Fatalities",
                                    "Workforce %",
                                ],
                                EsgPillar::Governance => &[
                                    "POSH Upheld",
                                    "Total POSH",
                                    "Filed",
                                    "Pending",
                                    "Disciplinary",
                                    "Well-Being %",
                                ],
                            };

                            for h in metric_headers {
                                header.col(|ui| {
                                    ui.label(RichText::new(*h).strong().color(header_color));
                                });
                            }
                            header.col(|ui| {
                                ui.label(RichText::new("Action").strong().color(header_color));
                            });
                        })
                        .body(|body| {
                            body.rows(24.0, filtered.len(), |mut row| {
                                let m = &filtered[row.index()];
                                row.col(|ui| {
                                    ui.add(
                                        egui::Label::new(RichText::new(&m.company_name).strong())
                                            .truncate(),
                                    )
                                    .on_hover_text(&m.corporate_id_number);
                                });
                                row.col(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&m.industry_sector)
                                                .small()
                                                .color(Color32::from_rgb(160, 170, 190)),
                                        )
                                        .truncate(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.add(
                                        egui::Label::new(RichText::new(&m.fy_label).small())
                                            .truncate(),
                                    );
                                });

                                match pillar {
                                    EsgPillar::Environmental => {
                                        for cell in [
                                            fmt_opt(m.scope_1_tco2e),
                                            fmt_opt(m.scope_2_tco2e),
                                            fmt_opt(m.scope_3_tco2e),
                                            fmt_opt_pct(m.renewable_energy_pct),
                                            fmt_opt(m.water_withdrawal_kl),
                                            fmt_opt(m.energy_intensity),
                                            fmt_opt(m.waste_intensity),
                                        ] {
                                            row.col(|ui| {
                                                ui.label(cell);
                                            });
                                        }
                                    }
                                    EsgPillar::Social => {
                                        for cell in [
                                            fmt_opt_pct(m.female_board_directors_pct),
                                            fmt_opt_pct(m.gross_wages_to_female_pct),
                                            fmt_opt_pct(m.employee_turnover_pct),
                                            fmt_opt(m.work_related_injuries),
                                            fmt_opt(m.fatalities),
                                            fmt_opt_pct(m.female_workforce_pct),
                                        ] {
                                            row.col(|ui| {
                                                ui.label(cell);
                                            });
                                        }
                                    }
                                    EsgPillar::Governance => {
                                        for cell in [
                                            fmt_opt(m.posh_complaints_upheld),
                                            fmt_opt(m.total_posh_complaints),
                                            fmt_opt(m.complaints_filed),
                                            fmt_opt(m.pending_complaints),
                                            fmt_opt(m.disciplinary_actions),
                                            fmt_opt_pct(m.cost_on_wellbeing_pct),
                                        ] {
                                            row.col(|ui| {
                                                ui.label(cell);
                                            });
                                        }
                                    }
                                }

                                row.col(|ui| {
                                    if ui.small_button("📄 View Form").clicked() {
                                        clicked_view =
                                            Some((m.corporate_id_number.clone(), m.entity_id));
                                    }
                                });
                            });
                        });
                });
        });

    if let Some((cin, eid)) = clicked_view {
        state.form_viewer.selected_cin = cin;
        state.form_viewer.selected_entity_id = Some(eid);
        state.active_tab = ActiveTab::FormViewer;
    }
}

fn fmt_opt(val: Option<f64>) -> String {
    val.map(|v| {
        if v != 0.0 && v.abs() < 0.1 {
            format!("{:.4}", v)
        } else {
            format!("{:.2}", v)
        }
    })
    .unwrap_or_else(|| "—".to_string())
}

fn fmt_opt_pct(val: Option<f64>) -> String {
    val.map(|v| format!("{:.1}%", v))
        .unwrap_or_else(|| "—".to_string())
}
