// src/ui/views/form_viewer/section_b_matrix.rs

use crate::ui::state::{AppState, FormFactRow, FormMasterItem};
use eframe::egui::{self, Color32, FontId, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use egui_extras::{Column, TableBuilder};
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

type SectionBGroupKey = (i16, String, String, String);
type SectionBGroupVal<'a> = (String, Vec<&'a FormMasterItem>);

pub fn render(
    ui: &mut Ui,
    state: &AppState,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    search_lower: &str,
) {
    let mut by_row_group: BTreeMap<SectionBGroupKey, SectionBGroupVal<'_>> = BTreeMap::new();

    let mut active_heading = String::new();
    let mut active_q = -1i16;
    let mut active_followup = String::new();

    for item in &state.form_viewer.form_template {
        if item.section != "B" || matches!(item.question_number, 7..=9) {
            continue;
        }

        if item.question_number != active_q || item.followup_question_first != active_followup {
            active_q = item.question_number;
            active_followup = item.followup_question_first.clone();
            active_heading.clear();
        }

        let has_tag = !item.standard_indicator_capmkt.trim().is_empty()
            && item.standard_indicator_capmkt.trim() != "0";

        if !has_tag {
            let cleaned = strip_leading_numbers(&item.question_text);
            if cleaned.eq_ignore_ascii_case("not applicable reason") {
                active_heading = format!("{} — Not Applicable Reason", active_heading);
            } else if active_heading.is_empty() {
                active_heading =
                    match (item.question_in_form, item.followup_question_first.as_str()) {
                        (Some(q), "0") | (Some(q), "") => format!("{}. {}", q, cleaned),
                        (Some(q), fu) => format!("{}({}). {}", q, fu, cleaned),
                        _ => cleaned.to_string(),
                    };
            } else if !cleaned.eq_ignore_ascii_case("subject for review")
                && !cleaned.eq_ignore_ascii_case("details of review of ngrbcs by the company")
            {
                active_heading = format!("{} — {}", active_heading, cleaned);
            }
            continue;
        }

        if item.field_flag == 2 {
            let group_key = (
                item.question_number,
                item.followup_question_first.clone(),
                item.standard_indicator_capmkt.clone(),
                active_heading.clone(),
            );
            by_row_group
                .entry(group_key)
                .or_insert_with(|| (active_heading.clone(), Vec::new()))
                .1
                .push(item);
        }
    }

    let mut render_rows: Vec<(String, [Option<&FormMasterItem>; 9])> = Vec::new();
    for (_key, (label, items)) in by_row_group {
        if !search_lower.is_empty() && !label.to_ascii_lowercase().contains(search_lower) {
            let any_fact_matches = items.iter().any(|m| {
                let std_ind = m.standard_indicator_capmkt.trim();
                let std_ctx = m.standard_context_reference.trim();
                exact_facts
                    .get(&(std_ind, std_ctx))
                    .and_then(|v| v.first())
                    .is_some_and(|f| f.raw_value.to_ascii_lowercase().contains(search_lower))
            });
            if !any_fact_matches {
                continue;
            }
        }

        let mut row_principles: [Option<&FormMasterItem>; 9] = [None; 9];
        for item in items {
            if let Some(p) = get_principle_number(item)
                && (1..=9).contains(&p)
            {
                row_principles[p - 1] = Some(item);
            }
        }
        render_rows.push((label, row_principles));
    }

    Frame::NONE
        .fill(Color32::from_rgb(22, 25, 32))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 52, 65)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.heading(
                RichText::new("Section B: Management & Process Disclosures (P1–P9 Matrix)")
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "Cross-Principle governance policies, board approvals, and commitments:",
                )
                .small()
                .color(Color32::GRAY),
            );
            ui.add_space(8.0);

            if render_rows.is_empty() {
                ui.label(
                    RichText::new("No matching Section B matrix entries.").color(Color32::GRAY),
                );
                return;
            }

            let body_font = egui::TextStyle::Body.resolve(ui.style());
            let q_col_w = 320.0;
            let p_col_w = 78.0;

            let row_heights: Vec<f32> = render_rows
                .iter()
                .map(|(label, cols)| {
                    let q_h = measure_wrapped_height(ui, label, body_font.clone(), q_col_w);
                    let mut max_p_h = 26.0f32;
                    for m_opt in cols.iter().flatten() {
                        let std_ind = m_opt.standard_indicator_capmkt.trim();
                        let std_ctx = m_opt.standard_context_reference.trim();
                        if let Some(f) = exact_facts
                            .get(&(std_ind, std_ctx))
                            .and_then(|v| v.first().copied())
                        {
                            let val = f.raw_value.trim();
                            if !val.starts_with("http") && val.chars().count() > 12 {
                                let h = measure_wrapped_height(
                                    ui,
                                    val,
                                    FontId::proportional(12.0),
                                    p_col_w,
                                );
                                max_p_h = max_p_h.max(h.min(72.0));
                            }
                        }
                    }
                    q_h.max(max_p_h).max(28.0)
                })
                .collect();

            ScrollArea::horizontal()
                .id_salt("section_b_horizontal_scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let mut table = TableBuilder::new(ui)
                        .id_salt("section_b_table_builder")
                        .striped(true)
                        .vscroll(false)
                        .auto_shrink([true, true])
                        .column(Column::exact(q_col_w));

                    for _ in 0..9 {
                        table = table.column(Column::exact(p_col_w));
                    }

                    table
                        .header(26.0, |mut header| {
                            header.col(|ui| {
                                ui.label(
                                    RichText::new("Statutory Governance Question")
                                        .strong()
                                        .color(Color32::from_rgb(100, 180, 240)),
                                );
                            });
                            for p in 1..=9 {
                                header.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("P{}", p))
                                            .strong()
                                            .color(Color32::from_rgb(120, 200, 130)),
                                    );
                                });
                            }
                        })
                        .body(|body| {
                            body.heterogeneous_rows(row_heights.into_iter(), |mut row| {
                                let (ref label, ref row_principles) = render_rows[row.index()];
                                row.col(|ui| {
                                    ui.add(egui::Label::new(RichText::new(label).strong()).wrap());
                                });

                                for (p_idx, m_opt) in row_principles.iter().enumerate() {
                                    let fact_val = m_opt.and_then(|m| {
                                        let std_ind = m.standard_indicator_capmkt.trim();
                                        let std_ctx = m.standard_context_reference.trim();
                                        exact_facts
                                            .get(&(std_ind, std_ctx))
                                            .and_then(|v| v.first().copied())
                                            .or_else(|| {
                                                tag_facts.get(std_ind).and_then(|list| {
                                                    let patterns = PRINCIPLE_PATTERNS[p_idx];
                                                    list.iter()
                                                        .find(|f| {
                                                            patterns.iter().any(|pat| {
                                                                f.context_ref.contains(pat)
                                                            })
                                                        })
                                                        .copied()
                                                })
                                            })
                                    });

                                    row.col(|ui| {
                                        if let Some(f) = fact_val {
                                            render_badge_or_text(ui, &f.raw_value);
                                        } else {
                                            ui.label(RichText::new("—").color(Color32::DARK_GRAY));
                                        }
                                    });
                                }
                            });
                        });
                });
        });
}

fn measure_wrapped_height(ui: &Ui, text: &str, font_id: FontId, width: f32) -> f32 {
    if text.is_empty() {
        return 24.0;
    }
    let h = ui
        .painter()
        .layout(
            text.to_string(),
            font_id,
            Color32::WHITE,
            (width - 8.0).max(40.0),
        )
        .size()
        .y;
    (h + 10.0).max(26.0)
}

#[inline]
fn get_principle_number(item: &FormMasterItem) -> Option<usize> {
    let ctx = item.standard_context_reference.as_str();
    for (idx, patterns) in PRINCIPLE_PATTERNS.iter().enumerate() {
        if patterns.iter().any(|pat| ctx.contains(pat)) {
            return Some(idx + 1);
        }
    }
    if (1..=9).contains(&item.principle_number) {
        return Some(item.principle_number as usize);
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
    } else if t.starts_with("http") {
        ui.hyperlink_to(
            RichText::new("Link ↗")
                .small()
                .color(Color32::from_rgb(110, 185, 250)),
            t,
        )
        .on_hover_text(t);
    } else if t.chars().count() > 16 {
        let preview: String = t.chars().take(14).collect();
        ui.label(RichText::new(format!("{}...", preview)).small())
            .on_hover_text(t);
    } else {
        ui.label(RichText::new(t).small());
    }
}
