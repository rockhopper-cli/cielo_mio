// src/ui/views/form_viewer/question_cards.rs

use super::components::{render_indicator_badge, render_singular_box};
use super::formatter::humanize_context;
use super::profile::resolve_profile_value;
use super::tables;
use super::visibility::{
    build_question_gates, build_response_index, is_visibility_satisfied, lookup_item_facts,
};

use crate::ui::state::{AppState, CompanyFilingSummary, FormFactRow, FormMasterItem};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Stroke, Ui};
use std::collections::{BTreeMap, HashMap};

pub fn is_essential(ind_type: &str, ind_text: &str) -> bool {
    let txt = ind_text.trim().to_ascii_lowercase();
    if txt.contains("essential") {
        return true;
    }
    if txt.contains("leadership") {
        return false;
    }
    let t = ind_type.trim();
    t.eq_ignore_ascii_case("e") || t.eq_ignore_ascii_case("essential")
}

pub fn is_leadership(ind_type: &str, ind_text: &str) -> bool {
    let txt = ind_text.trim().to_ascii_lowercase();
    if txt.contains("leadership") {
        return true;
    }
    if txt.contains("essential") {
        return false;
    }
    let t = ind_type.trim();
    t.eq_ignore_ascii_case("l") || t.eq_ignore_ascii_case("leadership")
}

pub fn get_indicator_label<'a>(ind_type: &'a str, ind_text: &'a str) -> &'a str {
    let txt = ind_text.trim();
    if !txt.is_empty() && !txt.eq_ignore_ascii_case("na") && !txt.eq_ignore_ascii_case("general") {
        txt
    } else if is_essential(ind_type, ind_text) {
        "Essential Indicator"
    } else if is_leadership(ind_type, ind_text) {
        "Leadership Indicator"
    } else {
        ind_type.trim()
    }
}

pub fn render(
    ui: &mut Ui,
    state: &AppState,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    search_lower: &str,
) {
    let current_profile: Option<&CompanyFilingSummary> =
        state.form_viewer.selected_entity_id.and_then(|ent_id| {
            state
                .form_viewer
                .available_filings
                .iter()
                .find(|f| f.entity_id == ent_id)
        });

    let key_responses = build_response_index(
        &state.form_viewer.form_template,
        exact_facts,
        tag_facts,
        current_profile,
    );
    let question_gates = build_question_gates(&state.form_viewer.form_template);

    let filter_essential = state
        .form_viewer
        .selected_indicator_type
        .as_deref()
        .is_some_and(|f| is_essential(f, ""));
    let filter_leadership = state
        .form_viewer
        .selected_indicator_type
        .as_deref()
        .is_some_and(|f| is_leadership(f, ""));

    // Group all visible items into question cards BEFORE applying search filtering so tables stay intact
    let mut grouped_questions: BTreeMap<(i16, bool, i16), Vec<&FormMasterItem>> = BTreeMap::new();

    for item in &state.form_viewer.form_template {
        if item.section != state.form_viewer.selected_section {
            continue;
        }
        // In Section B, Q1-6 and Q10-12 render in the 9-principle TableBuilder matrix; only Q7-9 render as cards
        if item.section == "B" && !matches!(item.question_number, 7..=9) {
            continue;
        }
        if item.section == "A"
            && state.form_viewer.selected_sub_section > 0
            && item.sub_section != state.form_viewer.selected_sub_section
        {
            continue;
        }
        if item.section == "C" && item.principle_number != state.form_viewer.selected_principle {
            continue;
        }

        if !is_visibility_satisfied(item, &question_gates, &key_responses) {
            continue;
        }

        let is_lead = is_leadership(&item.indicator_type, &item.indicator_text);
        let is_ess = is_essential(&item.indicator_type, &item.indicator_text);

        if filter_essential && !is_ess {
            continue;
        }
        if filter_leadership && !is_lead {
            continue;
        }

        grouped_questions
            .entry((item.sub_section, is_lead, item.question_number))
            .or_default()
            .push(item);
    }

    // Apply search filter at the question-card level so matching a line item keeps the full table intact
    if !search_lower.is_empty() {
        grouped_questions.retain(|_, items| {
            items.iter().any(|item| {
                item.question_text
                    .to_ascii_lowercase()
                    .contains(search_lower)
                    || item
                        .standard_indicator_capmkt
                        .to_ascii_lowercase()
                        .contains(search_lower)
                    || item
                        .input_text
                        .as_deref()
                        .is_some_and(|h| h.to_ascii_lowercase().contains(search_lower))
                    || lookup_item_facts(item, exact_facts, tag_facts)
                        .iter()
                        .any(|f| f.raw_value.to_ascii_lowercase().contains(search_lower))
                    || resolve_profile_value(item, current_profile)
                        .is_some_and(|v| v.to_ascii_lowercase().contains(search_lower))
            })
        });
    }

    if grouped_questions.is_empty() {
        if state.form_viewer.selected_section != "B" {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(
                    RichText::new("No statutory entries found for this selection.")
                        .color(Color32::GRAY),
                );
            });
        }
        return;
    }

    for (_key, items) in grouped_questions {
        let first_item = items[0];

        Frame::NONE
            .fill(Color32::from_rgb(22, 26, 35))
            .stroke(Stroke::new(1.0, Color32::from_rgb(40, 48, 62)))
            .corner_radius(6)
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                if first_item.section == "C" {
                    let is_ess =
                        is_essential(&first_item.indicator_type, &first_item.indicator_text);
                    let is_lead =
                        is_leadership(&first_item.indicator_type, &first_item.indicator_text);
                    let badge_label =
                        get_indicator_label(&first_item.indicator_type, &first_item.indicator_text);
                    render_indicator_badge(ui, is_ess, is_lead, badge_label);
                }

                let clean_title = strip_leading_numbers(&first_item.question_text);
                let header_title = match first_item.question_in_form {
                    Some(in_form) => format!("Q{}. {}", in_form, clean_title),
                    None => clean_title.to_string(),
                };

                ui.add(
                    egui::Label::new(
                        RichText::new(&header_title)
                            .strong()
                            .size(14.5)
                            .color(Color32::WHITE),
                    )
                    .wrap(),
                );

                if let Some(ref hint) = first_item.input_text
                    && !hint.trim().is_empty()
                {
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(hint)
                                .small()
                                .italics()
                                .color(Color32::from_rgb(150, 160, 175)),
                        )
                        .wrap(),
                    );
                }

                ui.add_space(8.0);

                // Render as BRSR egui_extras::TableBuilder if multi-item or dynamic axis table
                let is_table = items.len() > 1 || items.iter().any(|it| it.contextref_increment);

                if is_table {
                    tables::render_brsr_tables(
                        ui,
                        &items,
                        exact_facts,
                        tag_facts,
                        current_profile,
                        strip_leading_numbers,
                    );
                } else {
                    render_singular_item(ui, first_item, exact_facts, tag_facts, current_profile);
                }
            });

        ui.add_space(8.0);
    }
}

fn render_singular_item(
    ui: &mut Ui,
    first_item: &FormMasterItem,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
) {
    let facts = lookup_item_facts(first_item, exact_facts, tag_facts);

    let profile_val = if facts.is_empty() {
        resolve_profile_value(first_item, current_profile)
    } else {
        None
    };

    if let Some(val_str) = profile_val {
        render_singular_box(
            ui,
            &val_str,
            None,
            &None,
            first_item.append_to_field.as_deref(),
            "",
            &first_item.response_datatype,
        );
    } else if !facts.is_empty() {
        for f in facts {
            let ctx_label = humanize_context(&f.context_ref);
            render_singular_box(
                ui,
                &f.raw_value,
                f.standard_numeric_value,
                &f.standard_unit,
                first_item.append_to_field.as_deref(),
                ctx_label,
                &first_item.response_datatype,
            );
        }
    } else {
        ui.label(RichText::new("— Not Disclosed").color(Color32::from_rgb(120, 128, 142)));
    }
}

pub fn strip_leading_numbers(s: &str) -> &str {
    let trimmed = s.trim();
    let mut rest = trimmed;
    if rest.starts_with('Q') || rest.starts_with('q') {
        let after_q = &rest[1..];
        if after_q.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            rest = after_q;
        }
    }

    let mut chars = rest.chars();
    let mut digits = 0;
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            digits += 1;
        } else if (c == '.' || c == ')' || c == ':') && digits > 0 {
            return chars.as_str().trim_start();
        } else {
            break;
        }
    }
    trimmed
}
