// src/ui/views/form_viewer/report_view.rs

use super::{question_cards, section_b_matrix};
use crate::ui::state::AppState;
use eframe::egui::{
    self, Color32, Frame, Margin, RichText, ScrollArea, Ui, scroll_area::ScrollBarVisibility,
};
use std::collections::HashMap;

pub fn render(ui: &mut Ui, state: &mut AppState, _max_height: f32) {
    if state.form_viewer.is_loading_facts {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.spinner();
            ui.label("Rendering official disclosure report from repository...");
        });
        return;
    }

    if state.form_viewer.selected_entity_id.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new(
                    "Select a company and financial year above to inspect its BRSR report.",
                )
                .color(Color32::GRAY),
            );
        });
        return;
    }

    let fact_count = state.form_viewer.current_facts.len();
    let mut exact_facts: HashMap<(&str, &str), Vec<&crate::ui::state::FormFactRow>> =
        HashMap::with_capacity(fact_count);
    let mut tag_facts: HashMap<&str, Vec<&crate::ui::state::FormFactRow>> =
        HashMap::with_capacity(fact_count);

    for f in &state.form_viewer.current_facts {
        let ind = f.standard_indicator.trim();
        let ctx = f.context_ref.trim();
        exact_facts.entry((ind, ctx)).or_default().push(f);
        tag_facts.entry(ind).or_default().push(f);
    }

    let search_lower = state.form_viewer.search_query.trim().to_lowercase();

    let active_section_text = state
        .form_viewer
        .form_template
        .iter()
        .find(|m| m.section == state.form_viewer.selected_section)
        .map(|m| m.section_text.as_str())
        .unwrap_or("SECTION");

    Frame::NONE
        .fill(Color32::from_rgb(22, 26, 35))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 45, 58)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(
                    RichText::new(active_section_text)
                        .strong()
                        .size(16.0)
                        .color(Color32::WHITE),
                );

                if state.form_viewer.selected_section == "C"
                    && let Some(ref ind) = state.form_viewer.selected_indicator_type
                {
                    let is_ess = question_cards::is_essential(ind, "");
                    let is_lead = question_cards::is_leadership(ind, "");

                    let (bg, fg, label) = if is_ess {
                        (
                            Color32::from_rgb(20, 55, 30),
                            Color32::from_rgb(90, 220, 130),
                            "Essential Indicators",
                        )
                    } else if is_lead {
                        (
                            Color32::from_rgb(25, 45, 75),
                            Color32::from_rgb(100, 180, 240),
                            "Leadership Indicators",
                        )
                    } else {
                        (
                            Color32::from_rgb(40, 45, 55),
                            Color32::LIGHT_GRAY,
                            ind.as_str(),
                        )
                    };

                    Frame::NONE
                        .fill(bg)
                        .corner_radius(4)
                        .inner_margin(Margin::symmetric(7, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(label).small().strong().color(fg));
                        });
                }
            });

            if state.form_viewer.selected_section == "A"
                && state.form_viewer.selected_sub_section > 0
            {
                if let Some(sub_item) = state.form_viewer.form_template.iter().find(|m| {
                    m.section == "A" && m.sub_section == state.form_viewer.selected_sub_section
                }) {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("Sub-Section: {}", sub_item.sub_section_text))
                            .strong()
                            .size(13.0)
                            .color(Color32::from_rgb(100, 180, 240)),
                    );
                }
            } else if state.form_viewer.selected_section == "C"
                && state.form_viewer.selected_principle > 0
                && let Some(p_item) = state.form_viewer.form_template.iter().find(|m| {
                    m.section == "C" && m.principle_number == state.form_viewer.selected_principle
                })
            {
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(&p_item.principle_text)
                            .small()
                            .color(Color32::from_rgb(175, 185, 200)),
                    )
                    .wrap(),
                );
            }
        });

    ui.add_space(6.0);

    ScrollArea::vertical()
        .id_salt("report_body_scroll_area")
        .auto_shrink([false, false])
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            // Right padding prevents cards from touching the scrollbar edge
            Frame::NONE
                .inner_margin(Margin {
                    left: 2,
                    right: 12,
                    top: 2,
                    bottom: 8,
                })
                .show(ui, |ui| {
                    if state.form_viewer.selected_section == "B" {
                        section_b_matrix::render(
                            ui,
                            state,
                            &exact_facts,
                            &tag_facts,
                            &search_lower,
                        );
                        ui.add_space(10.0);
                        // Also render Section B narrative/governance questions Q7, Q8, Q9
                        question_cards::render(ui, state, &exact_facts, &tag_facts, &search_lower);
                    } else {
                        question_cards::render(ui, state, &exact_facts, &tag_facts, &search_lower);
                    }
                    ui.add_space(24.0);
                });
        });
}
