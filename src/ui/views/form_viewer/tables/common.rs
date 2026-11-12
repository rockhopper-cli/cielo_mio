// src/ui/views/form_viewer/tables/common.rs

use super::super::formatter::render_value_cell;
use crate::ui::state::FormFactRow;
use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Clone)]
pub struct CellData {
    pub raw: String,
    pub num: Option<Decimal>,
    pub unit: Option<String>,
    pub datatype: String,
    pub suffix: Option<String>,
}

impl CellData {
    pub fn from_fact(f: &FormFactRow, datatype: &str, suffix: Option<&str>) -> Self {
        Self {
            raw: f.raw_value.clone(),
            num: f.standard_numeric_value,
            unit: f.standard_unit.clone(),
            datatype: datatype.to_string(),
            suffix: suffix.map(str::to_string),
        }
    }

    pub fn empty(datatype: &str) -> Self {
        Self {
            raw: String::new(),
            num: None,
            unit: None,
            datatype: datatype.to_string(),
            suffix: None,
        }
    }

    pub fn display_len(&self) -> usize {
        self.raw.chars().count()
    }
}

pub fn find_fact<'a>(
    exact: &HashMap<(&str, &str), Vec<&'a FormFactRow>>,
    tag: &str,
    ctx: &str,
) -> Option<&'a FormFactRow> {
    exact.get(&(tag, ctx)).and_then(|v| v.first().copied())
}

pub fn text_cell(text: &str) -> Option<CellData> {
    Some(CellData {
        raw: text.to_string(),
        num: None,
        unit: None,
        datatype: "string".to_string(),
        suffix: None,
    })
}

pub fn fact_cell(f: Option<&FormFactRow>, datatype: &str) -> Option<CellData> {
    f.map(|row| CellData::from_fact(row, datatype, None))
}

pub fn render_cell_data(ui: &mut Ui, cell: Option<&CellData>) {
    match cell {
        Some(c) if !c.raw.trim().is_empty() => {
            render_value_cell(ui, &c.raw, c.num, &c.unit, c.suffix.as_deref(), &c.datatype);
        }
        _ => {
            ui.label(RichText::new("—").color(Color32::from_rgb(120, 128, 142)));
        }
    }
}

pub fn measure_height(ui: &Ui, text: &str, width: f32, font_id: &FontId) -> f32 {
    if text.trim().is_empty() {
        return 24.0;
    }
    let h = ui
        .painter()
        .layout(
            text.to_string(),
            font_id.clone(),
            Color32::WHITE,
            (width - 10.0).max(40.0),
        )
        .size()
        .y;
    (h + 12.0).max(26.0)
}

pub fn render_matrix_table(
    ui: &mut Ui,
    id_salt: impl Hash + Debug,
    headers: &[&str],
    col_widths: &[f32],
    rows: &[Vec<Option<CellData>>],
) {
    if rows.is_empty() {
        ui.label(RichText::new("— Not Disclosed").color(Color32::from_rgb(120, 128, 142)));
        return;
    }

    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let header_color = Color32::from_rgb(100, 180, 240);

    let row_heights: Vec<f32> = rows
        .iter()
        .map(|row| {
            let mut max_h = 26.0f32;
            for (c_idx, cell_opt) in row.iter().enumerate() {
                if let Some(cell) = cell_opt
                    && !cell.raw.trim().is_empty()
                {
                    let w = col_widths.get(c_idx).copied().unwrap_or(110.0);
                    let approx_text = if cell.unit.is_some() {
                        format!("{} {}", cell.raw, cell.unit.as_deref().unwrap_or(""))
                    } else {
                        cell.raw.clone()
                    };
                    max_h = max_h.max(measure_height(ui, &approx_text, w, &font_id));
                }
            }
            max_h
        })
        .collect();

    ScrollArea::horizontal()
        .id_salt(("hscroll", &id_salt))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let mut table = TableBuilder::new(ui)
                .id_salt(("tbl", &id_salt))
                .striped(true)
                .vscroll(false)
                .auto_shrink([true, true]);

            for &w in col_widths {
                table = table.column(Column::exact(w));
            }

            table
                .header(26.0, |mut header| {
                    for h in headers {
                        header.col(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(*h).strong().size(12.5).color(header_color),
                                )
                                .wrap(),
                            );
                        });
                    }
                })
                .body(|body| {
                    body.heterogeneous_rows(row_heights.into_iter(), |mut row| {
                        let r_data = &rows[row.index()];
                        for cell_opt in r_data {
                            row.col(|ui| {
                                render_cell_data(ui, cell_opt.as_ref());
                            });
                        }
                    });
                });
        });
}
