// src/app/tab_form_viewer/question_cards.rs

pub use super::formatter::clean_text;
use super::formatter::{humanize_context, render_value_cell};
use super::profile::resolve_profile_value;

use crate::app::state::{AppState, CompanyFilingSummary, FormMasterItem};
use eframe::egui::{self, Color32, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2};
use std::collections::{BTreeMap, HashMap};

pub fn is_essential(ind_type: &str, ind_text: &str) -> bool {
    let t = ind_type.trim();
    let txt = ind_text.trim();
    t.eq_ignore_ascii_case("e")
        || t.eq_ignore_ascii_case("essential")
        || txt.to_ascii_lowercase().contains("essential")
}

pub fn is_leadership(ind_type: &str, ind_text: &str) -> bool {
    let t = ind_type.trim();
    let txt = ind_text.trim();
    t.eq_ignore_ascii_case("l")
        || t.eq_ignore_ascii_case("leadership")
        || txt.to_ascii_lowercase().contains("leadership")
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
    exact_facts: &HashMap<(&str, &str), Vec<&crate::app::state::FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&crate::app::state::FormFactRow>>,
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

    let filter_essential = state
        .form_viewer
        .selected_indicator_type
        .as_deref()
        .map_or(false, |f| is_essential(f, ""));
    let filter_leadership = state
        .form_viewer
        .selected_indicator_type
        .as_deref()
        .map_or(false, |f| is_leadership(f, ""));

    let mut grouped_questions: BTreeMap<(i16, bool, i16), Vec<&FormMasterItem>> = BTreeMap::new();

    for item in &state.form_viewer.form_template {
        if item.section != state.form_viewer.selected_section {
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

        let is_lead = is_leadership(&item.indicator_type, &item.indicator_text);
        let is_ess = is_essential(&item.indicator_type, &item.indicator_text);

        if filter_essential && !is_ess {
            continue;
        }
        if filter_leadership && !is_lead {
            continue;
        }

        if !search_lower.is_empty() {
            let matches = item
                .question_text
                .to_ascii_lowercase()
                .contains(search_lower)
                || item
                    .standard_indicator_capmkt
                    .to_ascii_lowercase()
                    .contains(search_lower)
                || exact_facts
                    .get(&(
                        item.standard_indicator_capmkt.trim(),
                        item.standard_context_reference.trim(),
                    ))
                    .or_else(|| tag_facts.get(item.standard_indicator_capmkt.trim()))
                    .map_or(false, |list| {
                        list.iter()
                            .any(|f| f.raw_value.to_ascii_lowercase().contains(search_lower))
                    });

            if !matches {
                continue;
            }
        }

        grouped_questions
            .entry((item.sub_section, is_lead, item.question_number))
            .or_default()
            .push(item);
    }

    if grouped_questions.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new("No statutory entries found for this selection.")
                    .color(Color32::GRAY),
            );
        });
        return;
    }

    for (_key, mut items) in grouped_questions {
        items.sort_unstable_by_key(|a| a.question_order);
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

                    let (badge_bg, badge_fg) = if is_ess {
                        (
                            Color32::from_rgb(20, 60, 35),
                            Color32::from_rgb(100, 230, 140),
                        )
                    } else if is_lead {
                        (
                            Color32::from_rgb(25, 50, 80),
                            Color32::from_rgb(120, 200, 255),
                        )
                    } else {
                        (Color32::from_rgb(40, 45, 55), Color32::LIGHT_GRAY)
                    };

                    let badge_label =
                        get_indicator_label(&first_item.indicator_type, &first_item.indicator_text);

                    egui::Frame::NONE
                        .fill(badge_bg)
                        .corner_radius(4)
                        .inner_margin(Margin::symmetric(7, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(badge_label)
                                    .size(11.0)
                                    .strong()
                                    .color(badge_fg),
                            );
                        });
                    ui.add_space(4.0);
                }

                let clean_title = strip_leading_numbers(&first_item.question_text);
                let header_title = if let Some(in_form) = first_item.question_in_form {
                    format!("Q{}. {}", in_form, clean_title)
                } else {
                    clean_title.to_string()
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

                if let Some(ref hint) = first_item.input_text {
                    if !hint.trim().is_empty() {
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
                }

                ui.add_space(8.0);

                let is_relational = first_item.field_flag == 1 || items.len() > 1;

                if is_relational {
                    let avail_inner = ui.available_width().max(260.0);
                    let col0_w = (avail_inner * 0.46).clamp(160.0, 380.0);
                    let col1_w = (avail_inner - col0_w - 14.0).max(120.0);

                    egui::Frame::NONE
                        .fill(Color32::from_rgb(30, 36, 48))
                        .corner_radius(4)
                        .inner_margin(Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(col0_w, 0.0),
                                    Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.label(
                                            RichText::new("Line Item / Parameter")
                                                .strong()
                                                .color(Color32::from_rgb(100, 180, 240)),
                                        );
                                    },
                                );
                                ui.add_space(10.0);
                                ui.allocate_ui_with_layout(
                                    Vec2::new(col1_w, 0.0),
                                    Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.label(
                                            RichText::new("Reported Response")
                                                .strong()
                                                .color(Color32::from_rgb(100, 180, 240)),
                                        );
                                    },
                                );
                            });
                        });

                    ui.add_space(3.0);

                    let mut row_idx = 0;
                    for item in &items {
                        let raw_line = if !item.question_text.is_empty()
                            && item.question_text != first_item.question_text
                        {
                            &item.question_text
                        } else {
                            &item.standard_indicator_capmkt
                        };
                        let line_label = strip_leading_numbers(raw_line);

                        let std_ind = item.standard_indicator_capmkt.trim();
                        let std_ctx = item.standard_context_reference.trim();

                        let facts = exact_facts.get(&(std_ind, std_ctx)).or_else(|| {
                            if !std_ind.is_empty() {
                                tag_facts.get(std_ind)
                            } else {
                                None
                            }
                        });

                        let profile_val = if facts.map_or(true, |v| v.is_empty()) {
                            resolve_profile_value(item, current_profile)
                        } else {
                            None
                        };

                        let row_bg = if row_idx % 2 == 1 {
                            Color32::from_rgb(26, 31, 41)
                        } else {
                            Color32::from_rgb(20, 24, 32)
                        };
                        row_idx += 1;

                        if let Some(val_str) = profile_val {
                            render_table_row(
                                ui,
                                col0_w,
                                col1_w,
                                row_bg,
                                line_label,
                                &val_str,
                                None,
                                &None,
                                None,
                                &item.response_datatype,
                            );
                        } else if let Some(matching) = facts {
                            if !matching.is_empty() {
                                for f in matching {
                                    let ctx_suffix = humanize_context(&f.context_ref);
                                    let row_label = if !ctx_suffix.is_empty()
                                        && !line_label
                                            .to_ascii_lowercase()
                                            .contains(&ctx_suffix.to_ascii_lowercase())
                                    {
                                        format!("{} ({})", line_label, ctx_suffix)
                                    } else {
                                        line_label.to_string()
                                    };

                                    render_table_row(
                                        ui,
                                        col0_w,
                                        col1_w,
                                        row_bg,
                                        &row_label,
                                        &f.raw_value,
                                        f.standard_numeric_value,
                                        &f.standard_unit,
                                        item.append_to_field.as_deref(),
                                        &item.response_datatype,
                                    );
                                }
                            } else {
                                render_not_disclosed_row(ui, col0_w, col1_w, row_bg, line_label);
                            }
                        } else {
                            render_not_disclosed_row(ui, col0_w, col1_w, row_bg, line_label);
                        }
                    }
                } else {
                    let std_ind = first_item.standard_indicator_capmkt.trim();
                    let std_ctx = first_item.standard_context_reference.trim();

                    let facts = exact_facts.get(&(std_ind, std_ctx)).or_else(|| {
                        if !std_ind.is_empty() {
                            tag_facts.get(std_ind)
                        } else {
                            None
                        }
                    });

                    let profile_val = if facts.map_or(true, |v| v.is_empty()) {
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
                    } else if let Some(matching) = facts {
                        if !matching.is_empty() {
                            for f in matching {
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
                            ui.label(
                                RichText::new("— Not Disclosed")
                                    .color(Color32::from_rgb(120, 128, 142)),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new("— Not Disclosed")
                                .color(Color32::from_rgb(120, 128, 142)),
                        );
                    }
                }
            });

        ui.add_space(8.0);
    }
}

pub fn strip_leading_numbers(s: &str) -> &str {
    let trimmed = s.trim();
    let mut rest = trimmed;
    if rest.starts_with('Q') || rest.starts_with('q') {
        let after_q = &rest[1..];
        if after_q.chars().next().map_or(false, |c| c.is_ascii_digit()) {
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

fn render_table_row(
    ui: &mut Ui,
    col0_w: f32,
    col1_w: f32,
    row_bg: Color32,
    label: &str,
    raw_val: &str,
    num_val: Option<rust_decimal::Decimal>,
    unit: &Option<String>,
    suffix: Option<&str>,
    datatype: &str,
) {
    egui::Frame::NONE
        .fill(row_bg)
        .corner_radius(3)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(col0_w, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(label)
                                    .color(Color32::from_rgb(215, 222, 235))
                                    .strong(),
                            )
                            .wrap(),
                        );
                    },
                );
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(col1_w, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| {
                        render_value_cell(ui, raw_val, num_val, unit, suffix, datatype);
                    },
                );
            });
        });
}

fn render_not_disclosed_row(ui: &mut Ui, col0_w: f32, col1_w: f32, row_bg: Color32, label: &str) {
    egui::Frame::NONE
        .fill(row_bg)
        .corner_radius(3)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(col0_w, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(label).color(Color32::from_rgb(170, 178, 190)),
                            )
                            .wrap(),
                        );
                    },
                );
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(col1_w, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(
                            RichText::new("— Not Disclosed")
                                .color(Color32::from_rgb(120, 128, 142)),
                        );
                    },
                );
            });
        });
}

fn render_singular_box(
    ui: &mut Ui,
    raw: &str,
    num: Option<rust_decimal::Decimal>,
    unit: &Option<String>,
    suffix: Option<&str>,
    ctx: &str,
    datatype: &str,
) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(26, 31, 41))
        .stroke(Stroke::new(1.0, Color32::from_rgb(38, 46, 60)))
        .corner_radius(4)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            if !ctx.is_empty() {
                ui.label(
                    RichText::new(ctx)
                        .small()
                        .color(Color32::from_rgb(130, 160, 200)),
                );
                ui.add_space(2.0);
            }
            render_value_cell(ui, raw, num, unit, suffix, datatype);
        });
    ui.add_space(4.0);
}
