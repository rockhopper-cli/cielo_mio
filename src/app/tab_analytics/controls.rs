// src/app/tab_analytics/controls.rs

use crate::app::state::{AppState, EnvironmentalMetric, EsgPillar, GovernanceMetric, SocialMetric};
use eframe::egui::{self, Color32, Margin, RichText, Stroke, TextEdit, Ui, Vec2};

pub fn render_brsr_pillar_cards(ui: &mut Ui, state: &mut AppState) {
    let spacing = 10.0;
    let total_w = ui.available_width().max(480.0);
    let card_w = ((total_w - spacing * 2.0) / 3.0).max(140.0);

    ui.horizontal(|ui| {
        render_pillar_card(
            ui,
            card_w,
            EsgPillar::Environmental,
            "🌱",
            "Environmental",
            "7 metrics tracked",
            Color32::from_rgb(76, 175, 80),
            state.analytics.selected_pillar == EsgPillar::Environmental,
            &mut state.analytics.selected_pillar,
        );

        ui.add_space(spacing);

        render_pillar_card(
            ui,
            card_w,
            EsgPillar::Social,
            "👥",
            "Social",
            "6 metrics tracked",
            Color32::from_rgb(66, 165, 245),
            state.analytics.selected_pillar == EsgPillar::Social,
            &mut state.analytics.selected_pillar,
        );

        ui.add_space(spacing);

        render_pillar_card(
            ui,
            card_w,
            EsgPillar::Governance,
            "🏢",
            "Governance",
            "6 metrics tracked",
            Color32::from_rgb(255, 183, 77),
            state.analytics.selected_pillar == EsgPillar::Governance,
            &mut state.analytics.selected_pillar,
        );
    });
}

fn render_pillar_card(
    ui: &mut Ui,
    card_w: f32,
    pillar: EsgPillar,
    icon: &str,
    title: &str,
    subtitle: &str,
    accent_color: Color32,
    is_active: bool,
    target: &mut EsgPillar,
) {
    let bg = if is_active {
        Color32::from_rgb(26, 32, 44)
    } else {
        Color32::from_rgb(18, 22, 30)
    };

    let stroke = if is_active {
        Stroke::new(1.5, accent_color)
    } else {
        Stroke::new(1.0, Color32::from_rgb(42, 50, 62))
    };

    // Render frame to obtain its exact bounded rectangle
    let frame_resp = egui::Frame::NONE
        .fill(bg)
        .stroke(stroke)
        .corner_radius(8)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(card_w);
            ui.horizontal_centered(|ui| {
                let icon_bg = if is_active {
                    Color32::from_rgb(32, 44, 58)
                } else {
                    Color32::from_rgb(26, 30, 40)
                };

                egui::Frame::NONE
                    .fill(icon_bg)
                    .corner_radius(16)
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.label(RichText::new(icon).size(18.0));
                    });

                ui.add_space(8.0);

                ui.vertical(|ui| {
                    let title_color = if is_active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(215, 222, 235)
                    };
                    ui.label(RichText::new(title).size(14.5).strong().color(title_color));
                    ui.label(
                        RichText::new(subtitle)
                            .small()
                            .color(Color32::from_rgb(150, 160, 175)),
                    );
                });
            });
        });

    // Interact on the frame's exact rect without extra allocate calls
    let resp = ui.interact(
        frame_resp.response.rect,
        ui.id().with(title),
        egui::Sense::click(),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        *target = pillar;
    }
}

pub fn render_search_and_metric_capsules(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        // Search Filter (Wraps cleanly if screen is narrow)
        ui.vertical(|ui| {
            ui.label(
                RichText::new("SEARCH")
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(140, 150, 165)),
            );
            ui.add_space(2.0);
            ui.add(
                TextEdit::singleline(&mut state.analytics.search_query)
                    .hint_text("🔍 Search companies...")
                    .desired_width(190.0),
            );
        });

        ui.add_space(14.0);

        // Metric Capsules: Uses horizontal_wrapped with fluid wrapping
        ui.vertical(|ui| {
            ui.label(
                RichText::new("METRIC")
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(140, 150, 165)),
            );
            ui.add_space(2.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                match state.analytics.selected_pillar {
                    EsgPillar::Environmental => {
                        render_metric_pill(
                            ui,
                            "Scope 1 Emissions",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::Scope1Emissions,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::Scope1Emissions
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Scope 2 Emissions",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::Scope2Emissions,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::Scope2Emissions
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Scope 3 Emissions",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::Scope3Emissions,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::Scope3Emissions
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Renewable Energy %",
                            state.analytics.selected_env_metric == EnvironmentalMetric::RenewableEnergy,
                            || state.analytics.selected_env_metric = EnvironmentalMetric::RenewableEnergy,
                        );
                        render_metric_pill(
                            ui,
                            "Water Withdrawal",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::WaterWithdrawal,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::WaterWithdrawal
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Energy Intensity",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::EnergyIntensity,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::EnergyIntensity
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Waste Intensity",
                            state.analytics.selected_env_metric
                                == EnvironmentalMetric::WasteIntensity,
                            || {
                                state.analytics.selected_env_metric =
                                    EnvironmentalMetric::WasteIntensity
                            },
                        );
                    }
                    EsgPillar::Social => {
                        render_metric_pill(
                            ui,
                            "Female Board Directors",
                            state.analytics.selected_soc_metric
                                == SocialMetric::FemaleBoardDirectors,
                            || {
                                state.analytics.selected_soc_metric =
                                    SocialMetric::FemaleBoardDirectors
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Gross Wages to Female",
                            state.analytics.selected_soc_metric == SocialMetric::GrossWagesToFemale,
                            || {
                                state.analytics.selected_soc_metric =
                                    SocialMetric::GrossWagesToFemale
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Employee Turnover Rate",
                            state.analytics.selected_soc_metric
                                == SocialMetric::EmployeeTurnoverRate,
                            || {
                                state.analytics.selected_soc_metric =
                                    SocialMetric::EmployeeTurnoverRate
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Work-Related Injuries",
                            state.analytics.selected_soc_metric
                                == SocialMetric::WorkRelatedInjuries,
                            || {
                                state.analytics.selected_soc_metric =
                                    SocialMetric::WorkRelatedInjuries
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Fatalities",
                            state.analytics.selected_soc_metric == SocialMetric::Fatalities,
                            || state.analytics.selected_soc_metric = SocialMetric::Fatalities,
                        );
                        render_metric_pill(
                            ui,
                            "Female Workforce",
                            state.analytics.selected_soc_metric == SocialMetric::FemaleWorkforce,
                            || state.analytics.selected_soc_metric = SocialMetric::FemaleWorkforce,
                        );
                    }
                    EsgPillar::Governance => {
                        render_metric_pill(
                            ui,
                            "POSH Complaints Upheld",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::PoshComplaintsUpheld,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::PoshComplaintsUpheld
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Total POSH Complaints",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::TotalPoshComplaints,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::TotalPoshComplaints
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Complaints Filed",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::ComplaintsFiled,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::ComplaintsFiled
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Pending Complaints",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::PendingComplaints,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::PendingComplaints
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Disciplinary Actions",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::DisciplinaryActions,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::DisciplinaryActions
                            },
                        );
                        render_metric_pill(
                            ui,
                            "Cost on Well-Being",
                            state.analytics.selected_gov_metric
                                == GovernanceMetric::CostOnWellBeing,
                            || {
                                state.analytics.selected_gov_metric =
                                    GovernanceMetric::CostOnWellBeing
                            },
                        );
                    }
                }
            });
        });
    });
}

fn render_metric_pill<F: FnOnce()>(ui: &mut Ui, label: &str, is_active: bool, select_fn: F) {
    let (bg, text_color, stroke) = if is_active {
        (
            Color32::from_rgb(10, 14, 20),
            Color32::WHITE,
            Stroke::new(1.5, Color32::from_rgb(220, 230, 245)),
        )
    } else {
        (
            Color32::from_rgb(24, 28, 38),
            Color32::from_rgb(195, 205, 220),
            Stroke::new(1.0, Color32::from_rgb(50, 58, 72)),
        )
    };

    let resp = ui.add(
        egui::Button::new(RichText::new(label).size(11.5).strong().color(text_color))
            .fill(bg)
            .stroke(stroke)
            .corner_radius(13.0)
            .min_size(Vec2::new(0.0, 24.0)),
    );

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        select_fn();
    }
}

pub fn get_active_metric_label(state: &AppState) -> &'static str {
    match state.analytics.selected_pillar {
        EsgPillar::Environmental => match state.analytics.selected_env_metric {
            EnvironmentalMetric::Scope1Emissions => "Scope 1 Emissions",
            EnvironmentalMetric::Scope2Emissions => "Scope 2 Emissions",
            EnvironmentalMetric::Scope3Emissions => "Scope 3 Emissions",
            EnvironmentalMetric::RenewableEnergy => "Renewable Energy %",
            EnvironmentalMetric::WaterWithdrawal => "Water Withdrawal",
            EnvironmentalMetric::EnergyIntensity => "Energy Intensity",
            EnvironmentalMetric::WasteIntensity => "Waste Intensity",
        },
        EsgPillar::Social => match state.analytics.selected_soc_metric {
            SocialMetric::FemaleBoardDirectors => "Female Board Directors",
            SocialMetric::GrossWagesToFemale => "Gross Wages to Female",
            SocialMetric::EmployeeTurnoverRate => "Employee Turnover Rate",
            SocialMetric::WorkRelatedInjuries => "Work-Related Injuries",
            SocialMetric::Fatalities => "Fatalities",
            SocialMetric::FemaleWorkforce => "Female Workforce",
        },
        EsgPillar::Governance => match state.analytics.selected_gov_metric {
            GovernanceMetric::PoshComplaintsUpheld => "POSH Complaints Upheld",
            GovernanceMetric::TotalPoshComplaints => "Total POSH Complaints",
            GovernanceMetric::ComplaintsFiled => "Complaints Filed",
            GovernanceMetric::PendingComplaints => "Pending Complaints",
            GovernanceMetric::DisciplinaryActions => "Disciplinary Actions",
            GovernanceMetric::CostOnWellBeing => "Cost on Well-Being",
        },
    }
}
