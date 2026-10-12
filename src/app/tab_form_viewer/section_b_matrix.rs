// src/app/tab_form_viewer/section_b_matrix.rs

use crate::app::state::{AppState, FormMasterItem};
use eframe::egui::{self, Color32, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use std::collections::{BTreeMap, HashMap};

const PRINCIPLE_PATTERNS: [&[&str]; 9] = [
    &["Principle1", "Principle_1", "Principle 1", "Axis1", "P1"],
    &["Principle2", "Principle_2", "Principle 2", "Axis2", "P2"],
    &["Principle3", "Principle_3", "Principle 3", "Axis3", "P3"],
    &["Principle4", "Principle_4", "Principle 4", "Axis4", "P4"],
    &["Principle5", "Principle_5", "Principle 5", "Axis5", "P5"],
    &["Principle6", "Principle_6", "Principle 6", "Axis6", "P6"],
    &["Principle7", "Principle_7", "Principle 7", "Axis7", "P7"],
    &["Principle8", "Principle_8", "Principle 8", "Axis8", "P8"],
    &["Principle9", "Principle_9", "Principle 9", "Axis9", "P9"],
];

pub fn render(
    ui: &mut Ui,
    state: &AppState,
    exact_facts: &HashMap<(&str, &str), Vec<&crate::app::state::FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&crate::app::state::FormFactRow>>,
) {
    let mut grouped_by_q: BTreeMap<i16, Vec<&FormMasterItem>> = BTreeMap::new();
    for item in &state.form_viewer.form_template {
        if item.section == "B" {
            let q_num = item.question_in_form.unwrap_or(item.question_number);
            grouped_by_q.entry(q_num).or_default().push(item);
        }
    }

    Frame::NONE
        .fill(Color32::from_rgb(22, 25, 32))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 52, 65)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.heading(RichText::new("Section B: Management and Process Disclosures").strong());
            ui.label(
                RichText::new("Cross-Principle governance policies, board approvals, and commitments (P1 through P9):")
                    .small()
                    .color(Color32::GRAY),
            );
            ui.add_space(8.0);

            ScrollArea::horizontal()
                .id_salt("section_b_horizontal_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("dynamic_section_b_grid")
                        .striped(true)
                        .min_col_width(70.0)
                        .spacing([14.0, 10.0])
                        .show(ui, |ui| {
                            // Header Row
                            ui.vertical(|ui| {
                                ui.set_width(360.0);
                                ui.label(
                                    RichText::new("Statutory Governance Question")
                                        .strong()
                                        .color(Color32::from_rgb(100, 180, 240)),
                                );
                            });

                            for p in 1..=9 {
                                ui.vertical(|ui| {
                                    ui.set_min_width(65.0);
                                    ui.label(
                                        RichText::new(format!("P{}", p))
                                            .strong()
                                            .color(Color32::from_rgb(120, 200, 130)),
                                    );
                                });
                            }
                            ui.end_row();

                            // Question Rows (Proper bounding box calculation prevents text overlap)
                            for (_q_num, items) in grouped_by_q {
                                let first = items[0];
                                let clean_q_text = strip_leading_numbers(&first.question_text);
                                let label = if let Some(in_form) = first.question_in_form {
                                    format!("{}. {}", in_form, clean_q_text)
                                } else {
                                    clean_q_text.to_string()
                                };

                                // Question cell: expands dynamically so multi-line text scales the entire grid row
                                ui.vertical(|ui| {
                                    ui.set_width(360.0);
                                    ui.add(
                                        egui::Label::new(RichText::new(&label).strong())
                                            .wrap(),
                                    );
                                });

                                let mut row_principles: [Option<&FormMasterItem>; 9] = [None; 9];
                                for item in &items {
                                    if let Some(p) = get_principle_number(item) {
                                        if (1..=9).contains(&p) {
                                            row_principles[p - 1] = Some(item);
                                        }
                                    }
                                }

                                for p in 1..=9 {
                                    let fact_val = if let Some(m) = row_principles[p - 1] {
                                        let std_ind = m.standard_indicator_capmkt.trim();
                                        let std_ctx = m.standard_context_reference.trim();

                                        exact_facts
                                            .get(&(std_ind, std_ctx))
                                            .and_then(|v| v.first().copied())
                                            .or_else(|| {
                                                tag_facts.get(std_ind).and_then(|list| {
                                                    let patterns = PRINCIPLE_PATTERNS[p - 1];
                                                    list.iter().find(|f| {
                                                        let c = f.context_ref.as_str();
                                                        patterns.iter().any(|pat| c.contains(pat))
                                                    }).copied()
                                                })
                                            })
                                    } else {
                                        None
                                    };

                                    ui.vertical(|ui| {
                                        ui.set_min_width(65.0);
                                        if let Some(f) = fact_val {
                                            render_badge_or_text(ui, &f.raw_value);
                                        } else {
                                            ui.label(RichText::new("—").color(Color32::DARK_GRAY));
                                        }
                                    });
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}

#[inline]
fn get_principle_number(item: &FormMasterItem) -> Option<usize> {
    if (1..=9).contains(&item.principle_number) {
        return Some(item.principle_number as usize);
    }
    if (1..=9).contains(&item.question_order) {
        return Some(item.question_order as usize);
    }

    let ctx = item.standard_context_reference.as_str();
    for (idx, patterns) in PRINCIPLE_PATTERNS.iter().enumerate() {
        if patterns.iter().any(|pat| ctx.contains(pat)) {
            return Some(idx + 1);
        }
    }
    None
}

#[inline]
fn strip_leading_numbers(s: &str) -> &str {
    let trimmed = s.trim();
    let mut chars = trimmed.chars();
    let mut digits = 0;
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            digits += 1;
        } else if (c == '.' || c == ')') && digits > 0 {
            return chars.as_str().trim_start();
        } else {
            break;
        }
    }
    trimmed
}

fn render_badge_or_text(ui: &mut Ui, text: &str) {
    let t = text.trim();
    if t.eq_ignore_ascii_case("yes") || t.eq_ignore_ascii_case("true") {
        ui.label(
            RichText::new("✓ Yes")
                .strong()
                .color(Color32::from_rgb(85, 220, 135)),
        );
    } else if t.eq_ignore_ascii_case("no") || t.eq_ignore_ascii_case("false") {
        ui.label(RichText::new("✗ No").color(Color32::from_rgb(240, 100, 100)));
    } else if text.starts_with("http") {
        ui.hyperlink_to(
            RichText::new("Link ↗")
                .small()
                .color(Color32::from_rgb(110, 185, 250)),
            text,
        )
        .on_hover_text(text);
    } else {
        // Zero-allocation UTF-8 length inspection: stops evaluating at the 17th character
        let mut char_iter = text.char_indices();
        let char_14_byte_idx = char_iter.nth(13).map(|(i, _)| i);
        let has_more_than_16 = char_iter.nth(2).is_some();

        if has_more_than_16 {
            if let Some(byte_idx) = char_14_byte_idx {
                let slice = &text[..byte_idx];
                ui.label(RichText::new(format!("{slice}...")).small())
                    .on_hover_text(text);
            } else {
                ui.label(RichText::new(text).small());
            }
        } else {
            ui.label(RichText::new(text).small());
        }
    }
}
