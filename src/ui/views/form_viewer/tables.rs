// src/ui/views/form_viewer/tables.rs

mod common;
mod generic;
mod section_a;
mod section_c;

use crate::ui::state::{CompanyFilingSummary, FormFactRow, FormMasterItem};
use eframe::egui::Ui;
use std::collections::HashMap;

/// Renders a BRSR question as one or more structured `egui_extras::TableBuilder` tables
/// based on SEBI's statutory layout.
pub fn render_brsr_tables(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
    strip_fn: fn(&str) -> &str,
) {
    let first = items[0];
    let table_salt = (
        first.section.as_str(),
        first.principle_number,
        first.sub_section,
        first.question_number,
    );

    if section_a::try_render(ui, items, exact_facts, tag_facts, table_salt) {
        return;
    }

    if section_c::try_render(ui, items, exact_facts, tag_facts, strip_fn, table_salt) {
        return;
    }

    if items.iter().any(|it| it.contextref_increment) {
        generic::render_dynamic_axis_sections(
            ui,
            items,
            exact_facts,
            tag_facts,
            current_profile,
            strip_fn,
            table_salt,
        );
        return;
    }

    if generic::has_cy_py_pairs(items) {
        generic::render_cy_py_table(
            ui,
            items,
            exact_facts,
            tag_facts,
            current_profile,
            strip_fn,
            table_salt,
        );
        return;
    }

    generic::render_two_col_table(
        ui,
        items,
        exact_facts,
        tag_facts,
        current_profile,
        strip_fn,
        table_salt,
    );
}
