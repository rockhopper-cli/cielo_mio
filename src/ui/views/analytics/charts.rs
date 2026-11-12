// src/app/tab_analytics/charts.rs

use eframe::egui::{Color32, Ui};
use egui_plot::{Bar, BarChart, Plot};

/// Two-bar comparison: Selected entity value vs Sector median
pub fn render_peer_comparison(ui: &mut Ui, plot_id: &str, company_val: f64, sector_median: f64) {
    let bar_company = Bar::new(0.5, company_val)
        .name("Selected Entity")
        .width(0.38);
    let bar_sector = Bar::new(1.3, sector_median)
        .name("Sector Median")
        .width(0.38);

    let chart_company =
        BarChart::new("Selected Entity", vec![bar_company]).color(Color32::from_rgb(80, 210, 120));
    let chart_sector =
        BarChart::new("Sector Median", vec![bar_sector]).color(Color32::from_rgb(100, 180, 240));

    Plot::new(plot_id)
        .height(180.0)
        .include_y(0.0)
        .include_y((company_val.max(sector_median) * 1.15).max(1.0))
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(chart_company);
            plot_ui.bar_chart(chart_sector);
        });
}

/// Generic reusable single-series bar chart for multi-year metrics
pub fn render_single_metric_chart(
    ui: &mut Ui,
    plot_id: &str,
    series_name: &str,
    bars: Vec<Bar>,
    color: Color32,
    height: f32,
) {
    let chart = BarChart::new(series_name, bars).color(color);

    Plot::new(plot_id)
        .height(height)
        .include_y(0.0)
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(chart);
        });
}
