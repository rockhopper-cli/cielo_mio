// src/ui/views/form_viewer/components.rs

use super::formatter::render_value_cell;
use eframe::egui::{self, Color32, Margin, RichText, Stroke, Ui};
use rust_decimal::Decimal;

pub fn render_singular_box(
    ui: &mut Ui,
    raw: &str,
    num: Option<Decimal>,
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

pub fn render_indicator_badge(ui: &mut Ui, is_essential: bool, is_leadership: bool, label: &str) {
    let (badge_bg, badge_fg) = if is_essential {
        (
            Color32::from_rgb(20, 60, 35),
            Color32::from_rgb(100, 230, 140),
        )
    } else if is_leadership {
        (
            Color32::from_rgb(25, 50, 80),
            Color32::from_rgb(120, 200, 255),
        )
    } else {
        (Color32::from_rgb(40, 45, 55), Color32::LIGHT_GRAY)
    };

    egui::Frame::NONE
        .fill(badge_bg)
        .corner_radius(4)
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).strong().color(badge_fg));
        });
    ui.add_space(4.0);
}

pub fn render_table_subheading(ui: &mut Ui, label: &str) {
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(Color32::from_rgb(28, 35, 48))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 58, 80)))
        .corner_radius(3)
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(label)
                        .strong()
                        .size(12.5)
                        .color(Color32::from_rgb(110, 190, 255)),
                )
                .wrap(),
            );
        });
    ui.add_space(2.0);
}
