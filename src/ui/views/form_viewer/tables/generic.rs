// src/ui/views/form_viewer/tables/generic.rs

use super::super::components::render_table_subheading;
use super::super::profile::resolve_profile_value;
use super::common::{
    CellData, fact_cell, find_fact, measure_height, render_cell_data, render_matrix_table,
    text_cell,
};
use crate::ui::state::{CompanyFilingSummary, FormFactRow, FormMasterItem};
use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

pub fn render_dynamic_axis_sections(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) {
    let mut sub_idx = 0usize;
    let mut i = 0usize;

    while i < items.len() {
        if !items[i].contextref_increment {
            let start = i;
            while i < items.len() && !items[i].contextref_increment {
                i += 1;
            }
            let slice = &items[start..i];
            if slice.iter().any(|it| {
                let tag = it.standard_indicator_capmkt.trim();
                !tag.is_empty() && tag != "0"
            }) {
                sub_idx += 1;
                render_two_col_table(
                    ui,
                    slice,
                    exact_facts,
                    tag_facts,
                    current_profile,
                    strip_fn,
                    (salt.0, salt.1, salt.2, salt.3 * 100 + sub_idx as i16),
                );
            }
            continue;
        }

        let axis_prefix = items[i]
            .standard_context_reference
            .trim()
            .trim_end_matches(|c: char| c.is_ascii_digit());
        let start = i;
        while i < items.len() && items[i].contextref_increment {
            let p = items[i]
                .standard_context_reference
                .trim()
                .trim_end_matches(|c: char| c.is_ascii_digit());
            if p != axis_prefix && !p.starts_with(axis_prefix) && !axis_prefix.starts_with(p) {
                break;
            }
            i += 1;
        }

        sub_idx += 1;
        let cols = &items[start..i];

        if start > 0
            && !items[start - 1].contextref_increment
            && (items[start - 1].standard_indicator_capmkt.trim().is_empty()
                || items[start - 1].standard_indicator_capmkt.trim() == "0")
        {
            let h = strip_fn(&items[start - 1].question_text);
            if !h.eq_ignore_ascii_case("s.no.") && !h.is_empty() && start - 1 > 0 {
                render_table_subheading(ui, h);
            }
        }

        let mut row_indices: BTreeSet<u32> = BTreeSet::new();
        for col in cols {
            let tag = col.standard_indicator_capmkt.trim();
            let col_prefix = col
                .standard_context_reference
                .trim()
                .trim_end_matches(|c: char| c.is_ascii_digit());
            if let Some(list) = tag_facts.get(tag) {
                for f in list {
                    if let Some(rest) = f.context_ref.trim().strip_prefix(col_prefix)
                        && !rest.is_empty()
                        && rest.chars().all(|c| c.is_ascii_digit())
                        && let Ok(n) = rest.parse::<u32>()
                    {
                        row_indices.insert(n);
                    }
                }
            }
        }

        if row_indices.is_empty() {
            row_indices.insert(1);
        }

        let mut headers: Vec<String> = Vec::with_capacity(cols.len() + 1);
        headers.push("S.No.".to_string());
        for col in cols {
            headers.push(strip_fn(&col.question_text).to_string());
        }

        let mut table_rows: Vec<Vec<Option<CellData>>> = Vec::with_capacity(row_indices.len());
        for (sno, r_idx) in row_indices.iter().enumerate() {
            let mut r_cells = Vec::with_capacity(cols.len() + 1);
            r_cells.push(text_cell(&format!("{}", sno + 1)));
            for col in cols {
                let tag = col.standard_indicator_capmkt.trim();
                let col_prefix = col
                    .standard_context_reference
                    .trim()
                    .trim_end_matches(|c: char| c.is_ascii_digit());
                let target_ctx = format!("{}{}", col_prefix, r_idx);
                let found = find_fact(exact_facts, tag, &target_ctx);
                r_cells.push(fact_cell(found, &col.response_datatype));
            }
            table_rows.push(r_cells);
        }

        let avail = ui.available_width().max(420.0);
        let sno_w = 46.0;
        let col_count = cols.len().max(1);
        let even_w = ((avail - sno_w - 18.0) / col_count as f32).clamp(115.0, 220.0);
        let mut widths = Vec::with_capacity(cols.len() + 1);
        widths.push(sno_w);
        for _ in 0..cols.len() {
            widths.push(even_w);
        }

        let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        render_matrix_table(
            ui,
            (salt, "axis", sub_idx),
            &header_refs,
            &widths,
            &table_rows,
        );
        ui.add_space(6.0);
    }
}

pub fn has_cy_py_pairs(items: &[&FormMasterItem]) -> bool {
    let mut has_cy = false;
    let mut has_py = false;
    for it in items {
        let q = it.question_text.trim();
        if q.eq_ignore_ascii_case("FY (Current Financial Year)")
            || it.standard_context_reference.trim() == "DCYMain"
        {
            has_cy = true;
        }
        if q.eq_ignore_ascii_case("FY (Previous Financial Year)")
            || it.standard_context_reference.trim() == "DPYMain"
            || it.standard_context_reference.trim().ends_with("_PY")
        {
            has_py = true;
        }
    }
    has_cy && has_py
}

fn is_fy_cy_label(text: &str) -> bool {
    text.trim()
        .eq_ignore_ascii_case("FY (Current Financial Year)")
}

fn is_fy_py_label(text: &str) -> bool {
    text.trim()
        .eq_ignore_ascii_case("FY (Previous Financial Year)")
}

pub fn render_cy_py_table(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) {
    let mut rows: Vec<Vec<Option<CellData>>> = Vec::new();
    let mut pending_other: Vec<&FormMasterItem> = Vec::new();
    let mut current_param = String::new();
    let mut idx = 0usize;

    while idx < items.len() {
        let item = items[idx];
        let tag = item.standard_indicator_capmkt.trim();
        let has_tag = !tag.is_empty() && tag != "0";
        let q_txt = strip_fn(&item.question_text);

        if !has_tag {
            if item.question_order > 1
                && !q_txt.eq_ignore_ascii_case("units")
                && !q_txt.eq_ignore_ascii_case("parameter")
                && !q_txt.eq_ignore_ascii_case("s.no.")
            {
                current_param = q_txt.to_string();
            }
            idx += 1;
            continue;
        }

        if idx + 1 < items.len() {
            let next = items[idx + 1];
            let next_tag = next.standard_indicator_capmkt.trim();
            let this_is_cy = is_fy_cy_label(&item.question_text)
                || item.standard_context_reference.trim() == "DCYMain";
            let next_is_py = is_fy_py_label(&next.question_text)
                || next.standard_context_reference.trim() == "DPYMain"
                || next.standard_context_reference.trim().ends_with("_PY");

            if tag == next_tag && this_is_cy && next_is_py {
                let param_label = if !current_param.is_empty() {
                    current_param.clone()
                } else {
                    tag.to_string()
                };
                let cy_f = find_fact(exact_facts, tag, item.standard_context_reference.trim());
                let py_f = find_fact(
                    exact_facts,
                    next_tag,
                    next.standard_context_reference.trim(),
                );
                rows.push(vec![
                    text_cell(&param_label),
                    fact_cell(cy_f, &item.response_datatype),
                    fact_cell(py_f, &next.response_datatype),
                ]);
                idx += 2;
                continue;
            }
        }

        pending_other.push(item);
        idx += 1;
    }

    if !rows.is_empty() {
        let avail = ui.available_width().max(420.0);
        let c0 = (avail * 0.48).clamp(180.0, 360.0);
        let cw = ((avail - c0 - 20.0) / 2.0).max(110.0);
        render_matrix_table(
            ui,
            (salt, "cy_py"),
            &["Parameter / Metric", "Current FY", "Previous FY"],
            &[c0, cw, cw],
            &rows,
        );
    }

    if !pending_other.is_empty() {
        ui.add_space(6.0);
        render_two_col_table(
            ui,
            &pending_other,
            exact_facts,
            tag_facts,
            current_profile,
            strip_fn,
            (salt.0, salt.1, salt.2, salt.3.saturating_add(500)),
        );
    }
}

pub fn render_two_col_table(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) {
    let mut rows: Vec<(String, Option<CellData>)> = Vec::new();

    for item in items {
        let std_ind = item.standard_indicator_capmkt.trim();
        let has_no_tag = std_ind == "0" || std_ind.is_empty();
        let is_heading = item.question_type.eq_ignore_ascii_case("question") && has_no_tag;

        if item.question_order == 1 && is_heading {
            continue;
        }

        let label = strip_fn(&item.question_text);
        if is_heading {
            if !rows.is_empty() {
                flush_two_col(ui, (salt, rows.len()), &rows);
                rows.clear();
            }
            render_table_subheading(ui, label);
            continue;
        }
        if has_no_tag {
            continue;
        }

        let std_ctx = item.standard_context_reference.trim();
        let facts = exact_facts
            .get(&(std_ind, std_ctx))
            .or_else(|| tag_facts.get(std_ind));

        let profile_val = if facts.is_none_or(|v| v.is_empty()) {
            resolve_profile_value(item, current_profile)
        } else {
            None
        };

        if let Some(val_str) = profile_val {
            rows.push((
                label.to_string(),
                Some(CellData {
                    raw: val_str.into_owned(),
                    num: None,
                    unit: None,
                    datatype: item.response_datatype.clone(),
                    suffix: None,
                }),
            ));
        } else if let Some(matching) = facts
            && !matching.is_empty()
        {
            for f in matching {
                rows.push((
                    label.to_string(),
                    Some(CellData::from_fact(
                        f,
                        &item.response_datatype,
                        item.append_to_field.as_deref(),
                    )),
                ));
            }
        } else {
            rows.push((
                label.to_string(),
                Some(CellData::empty(&item.response_datatype)),
            ));
        }
    }

    if !rows.is_empty() {
        flush_two_col(ui, (salt, 999usize), &rows);
    }
}

fn flush_two_col(ui: &mut Ui, salt: impl Hash + Debug, rows: &[(String, Option<CellData>)]) {
    let avail = ui.available_width().max(320.0);
    let col0_w = (avail * 0.45).clamp(150.0, 340.0);
    let col1_w = (avail - col0_w - 22.0).max(140.0);
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let header_color = Color32::from_rgb(100, 180, 240);

    let heights: Vec<f32> = rows
        .iter()
        .map(|(label, cell_opt)| {
            let h0 = measure_height(ui, label, col0_w, &font_id);
            let h1 = cell_opt
                .as_ref()
                .filter(|c| c.display_len() > 0)
                .map(|c| measure_height(ui, &c.raw, col1_w, &font_id))
                .unwrap_or(24.0);
            h0.max(h1).max(26.0)
        })
        .collect();

    TableBuilder::new(ui)
        .id_salt(("two_col", salt))
        .striped(true)
        .vscroll(false)
        .auto_shrink([true, true])
        .column(Column::exact(col0_w))
        .column(Column::exact(col1_w))
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.label(
                    RichText::new("Line Item / Parameter")
                        .strong()
                        .size(12.5)
                        .color(header_color),
                );
            });
            header.col(|ui| {
                ui.label(
                    RichText::new("Reported Response")
                        .strong()
                        .size(12.5)
                        .color(header_color),
                );
            });
        })
        .body(|body| {
            body.heterogeneous_rows(heights.into_iter(), |mut row| {
                let (ref label, ref cell_opt) = rows[row.index()];
                row.col(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(label)
                                .color(Color32::from_rgb(215, 222, 235))
                                .strong(),
                        )
                        .wrap(),
                    );
                });
                row.col(|ui| {
                    let is_empty = cell_opt.as_ref().is_none_or(|c| c.raw.trim().is_empty());
                    if is_empty {
                        ui.label(
                            RichText::new("— Not Disclosed")
                                .color(Color32::from_rgb(120, 128, 142)),
                        );
                    } else {
                        render_cell_data(ui, cell_opt.as_ref());
                    }
                });
            });
        });
}
