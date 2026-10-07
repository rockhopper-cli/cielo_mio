// src/app/tab_form_viewer/nav_tree.rs

use super::question_cards;
use crate::app::state::AppState;
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui, scroll_area::ScrollBarVisibility};
use std::collections::BTreeMap;

pub fn render(ui: &mut Ui, state: &mut AppState, _max_height: f32) {
    ui.label(
        RichText::new("📑 Form Navigation")
            .strong()
            .color(Color32::from_rgb(100, 180, 240)),
    );
    ui.add_space(4.0);

    if state.form_viewer.form_template.is_empty() {
        ui.label(
            RichText::new("Loading form structure...")
                .small()
                .color(Color32::GRAY),
        );
        return;
    }

    let mut section_a_subsections: BTreeMap<i16, &str> = BTreeMap::new();
    let mut section_c_principles: BTreeMap<i16, (&str, bool, bool)> = BTreeMap::new();
    let mut sec_a_name = "Section A: General Disclosures";
    let mut sec_b_name = "Section B: Management & Process";
    let mut sec_c_name = "Section C: Principle-wise Performance";

    for item in &state.form_viewer.form_template {
        match item.section.as_str() {
            "A" => {
                if !item.section_text.is_empty() {
                    sec_a_name = &item.section_text;
                }
                if item.sub_section > 0 {
                    section_a_subsections
                        .entry(item.sub_section)
                        .or_insert(&item.sub_section_text);
                }
            }
            "B" => {
                if !item.section_text.is_empty() {
                    sec_b_name = &item.section_text;
                }
            }
            "C" => {
                if !item.section_text.is_empty() {
                    sec_c_name = &item.section_text;
                }
                if item.principle_number > 0 {
                    let entry = section_c_principles
                        .entry(item.principle_number)
                        .or_insert((&item.principle_text, false, false));

                    if question_cards::is_essential(&item.indicator_type, &item.indicator_text) {
                        entry.1 = true;
                    } else if question_cards::is_leadership(
                        &item.indicator_type,
                        &item.indicator_text,
                    ) {
                        entry.2 = true;
                    }
                }
            }
            _ => {}
        }
    }

    ScrollArea::both()
        .id_salt("left_nav_tree_scroll_area")
        .auto_shrink([false, false])
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            // SECTION A
            let is_sec_a = state.form_viewer.selected_section == "A";

            egui::CollapsingHeader::new(RichText::new(sec_a_name).strong())
                .id_salt("header_section_a")
                .default_open(is_sec_a)
                .show(ui, |ui| {
                    let is_all_a = is_sec_a && state.form_viewer.selected_sub_section == 0;
                    if ui
                        .selectable_label(is_all_a, "• All Section A Details")
                        .clicked()
                    {
                        state.form_viewer.selected_section = "A".to_string();
                        state.form_viewer.selected_sub_section = 0;
                        state.form_viewer.selected_principle = 0;
                    }

                    for (sub_num, sub_title) in section_a_subsections {
                        let is_sub = is_sec_a && state.form_viewer.selected_sub_section == sub_num;
                        if ui
                            .selectable_label(is_sub, format!("• {}", sub_title))
                            .on_hover_text(sub_title)
                            .clicked()
                        {
                            state.form_viewer.selected_section = "A".to_string();
                            state.form_viewer.selected_sub_section = sub_num;
                            state.form_viewer.selected_principle = 0;
                        }
                    }
                });

            ui.add_space(4.0);

            // SECTION B
            let is_sec_b = state.form_viewer.selected_section == "B";
            if ui
                .selectable_label(is_sec_b, RichText::new(sec_b_name).strong())
                .on_hover_text(sec_b_name)
                .clicked()
            {
                state.form_viewer.selected_section = "B".to_string();
                state.form_viewer.selected_sub_section = 0;
                state.form_viewer.selected_principle = 0;
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // SECTION C
            ui.label(RichText::new(sec_c_name).small().color(Color32::GRAY));
            ui.add_space(2.0);

            for (p_num, (p_title, has_essential, has_leadership)) in section_c_principles {
                let is_p_active = state.form_viewer.selected_section == "C"
                    && state.form_viewer.selected_principle == p_num;

                let header_resp = egui::CollapsingHeader::new(
                    RichText::new(format!("P{}: {}", p_num, p_title)).strong(),
                )
                .id_salt(format!("header_principle_{}", p_num))
                .default_open(is_p_active)
                .show(ui, |ui| {
                    let is_all = is_p_active && state.form_viewer.selected_indicator_type.is_none();
                    if ui.selectable_label(is_all, "  • All Indicators").clicked() {
                        state.form_viewer.selected_section = "C".to_string();
                        state.form_viewer.selected_principle = p_num;
                        state.form_viewer.selected_sub_section = 0;
                        state.form_viewer.selected_indicator_type = None;
                    }

                    if has_essential {
                        let is_ess = is_p_active
                            && state
                                .form_viewer
                                .selected_indicator_type
                                .as_deref()
                                .map_or(false, |s| question_cards::is_essential(s, ""));

                        if ui
                            .selectable_label(is_ess, "  • Essential Indicators")
                            .clicked()
                        {
                            state.form_viewer.selected_section = "C".to_string();
                            state.form_viewer.selected_principle = p_num;
                            state.form_viewer.selected_sub_section = 0;
                            state.form_viewer.selected_indicator_type =
                                Some("Essential".to_string());
                        }
                    }

                    if has_leadership {
                        let is_lead = is_p_active
                            && state
                                .form_viewer
                                .selected_indicator_type
                                .as_deref()
                                .map_or(false, |s| question_cards::is_leadership(s, ""));

                        if ui
                            .selectable_label(is_lead, "  • Leadership Indicators")
                            .clicked()
                        {
                            state.form_viewer.selected_section = "C".to_string();
                            state.form_viewer.selected_principle = p_num;
                            state.form_viewer.selected_sub_section = 0;
                            state.form_viewer.selected_indicator_type =
                                Some("Leadership".to_string());
                        }
                    }
                });

                if header_resp.header_response.clicked() {
                    state.form_viewer.selected_section = "C".to_string();
                    state.form_viewer.selected_principle = p_num;
                    state.form_viewer.selected_sub_section = 0;
                    state.form_viewer.selected_indicator_type = None;
                }

                ui.add_space(2.0);
            }
        });
}
