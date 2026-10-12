// src/app/tab_form_viewer/formatter.rs

use eframe::egui::{self, Color32, RichText, Ui};

/// Formats values according to the statutory `response_datatype` (email, phone, number with commas, etc.)
pub fn render_value_cell(
    ui: &mut Ui,
    raw: &str,
    num: Option<rust_decimal::Decimal>,
    unit: &Option<String>,
    append_suffix: Option<&str>,
    datatype: &str,
) {
    let raw_trimmed = raw.trim();
    let clean_suffix = match append_suffix.unwrap_or("").trim() {
        "_PY" => " (Prior Year)",
        "_CY" => " (Current Year)",
        s => s,
    };

    let dt = datatype.trim().to_lowercase();

    // 1. Boolean formatting
    if dt == "boolean"
        || raw_trimmed.eq_ignore_ascii_case("yes")
        || raw_trimmed.eq_ignore_ascii_case("true")
        || raw_trimmed.eq_ignore_ascii_case("no")
        || raw_trimmed.eq_ignore_ascii_case("false")
    {
        if raw_trimmed.eq_ignore_ascii_case("yes") || raw_trimmed.eq_ignore_ascii_case("true") {
            ui.label(
                RichText::new("✓ Yes")
                    .strong()
                    .color(Color32::from_rgb(85, 220, 135)),
            );
            return;
        } else if raw_trimmed.eq_ignore_ascii_case("no")
            || raw_trimmed.eq_ignore_ascii_case("false")
        {
            ui.label(RichText::new("✗ No").color(Color32::from_rgb(240, 100, 100)));
            return;
        }
    }

    // 2. Email formatting (mailto link with mail icon)
    if dt == "email" || (raw_trimmed.contains('@') && !raw_trimmed.contains(' ')) {
        ui.hyperlink_to(
            RichText::new(format!("✉ {}", raw_trimmed))
                .color(Color32::from_rgb(110, 190, 255))
                .underline(),
            format!("mailto:{}", raw_trimmed),
        )
        .on_hover_text(format!("Click to email {}", raw_trimmed));
        return;
    }

    // 3. Phone formatting (telephone icon & clean spacing)
    if dt == "phone" {
        let clean_phone = format_phone(raw_trimmed);
        ui.label(
            RichText::new(format!("📞 {}", clean_phone))
                .strong()
                .color(Color32::from_rgb(130, 210, 170)),
        );
        return;
    }

    // 4. Date formatting (calendar icon)
    if dt == "date" {
        ui.label(
            RichText::new(format!("📅 {}", raw_trimmed)).color(Color32::from_rgb(220, 225, 235)),
        );
        return;
    }

    // 5. Number formatting (with thousands separators)
    if dt == "number" || num.is_some() {
        let formatted_num = if let Some(val) = num {
            format_decimal_with_commas(val)
        } else {
            format_numeric_str_with_commas(raw_trimmed)
        };

        let unit_str = match unit.as_deref() {
            Some(u) if u.eq_ignore_ascii_case("pure") || u.to_lowercase().starts_with("pure") => "",
            Some(u) => u,
            None => "",
        };

        let display = if !unit_str.is_empty() {
            format!("{} {}{}", formatted_num, unit_str, clean_suffix)
        } else {
            format!("{}{}", formatted_num, clean_suffix)
        };

        ui.add(
            egui::Label::new(
                RichText::new(display)
                    .strong()
                    .color(Color32::from_rgb(240, 200, 90)),
            )
            .wrap(),
        );
        return;
    }

    // 6. Hyperlink formatting
    if dt == "hyperlink"
        || raw_trimmed.starts_with("http://")
        || raw_trimmed.starts_with("https://")
    {
        let display_url = if raw_trimmed.len() > 36 {
            let boundary = raw_trimmed
                .char_indices()
                .nth(33)
                .map(|(i, _)| i)
                .unwrap_or(33);
            format!("{}... ↗", &raw_trimmed[..boundary])
        } else {
            format!("{} ↗", raw_trimmed)
        };
        ui.hyperlink_to(
            RichText::new(display_url)
                .color(Color32::from_rgb(110, 185, 250))
                .underline(),
            raw_trimmed,
        )
        .on_hover_text(raw_trimmed);
        return;
    }

    // 7. General String
    if clean_suffix.is_empty() {
        ui.add(
            egui::Label::new(RichText::new(raw_trimmed).color(Color32::from_rgb(215, 222, 235)))
                .wrap(),
        );
    } else {
        let display = format!("{}{}", raw_trimmed, clean_suffix);
        ui.add(
            egui::Label::new(RichText::new(display).color(Color32::from_rgb(215, 222, 235))).wrap(),
        );
    }
}

pub fn format_decimal_with_commas(val: rust_decimal::Decimal) -> String {
    let normalized = val.normalize().to_string();
    format_numeric_str_with_commas(&normalized)
}

pub fn format_numeric_str_with_commas(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (is_neg, body) = if let Some(stripped) = trimmed.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, trimmed)
    };

    let parts: Vec<&str> = body.splitn(2, '.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1);

    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return raw.to_string();
    }

    let digits: Vec<char> = int_part.chars().collect();
    let len = digits.len();
    let mut formatted_int = String::with_capacity(len + len / 3);

    for (i, ch) in digits.iter().enumerate() {
        formatted_int.push(*ch);
        let remaining = len - 1 - i;
        if remaining > 0 && remaining % 3 == 0 {
            formatted_int.push(',');
        }
    }

    let mut result = String::new();
    if is_neg {
        result.push('-');
    }
    result.push_str(&formatted_int);
    if let Some(dec) = dec_part {
        result.push('.');
        result.push_str(dec);
    }
    result
}

pub fn format_phone(raw: &str) -> String {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    if clean.len() == 10 && !clean.starts_with('+') {
        format!("+91 {} {}", &clean[0..5], &clean[5..10])
    } else if clean.len() == 12 && clean.starts_with("91") {
        format!("+91 {} {}", &clean[2..7], &clean[7..12])
    } else {
        raw.to_string()
    }
}

#[inline]
pub fn humanize_context(raw: &str) -> &str {
    let s = raw.trim();
    if s.contains("CY") || s.contains("CurrentYear") {
        "Current Year"
    } else if s.contains("PY") || s.contains("PreviousYear") {
        "Previous Year"
    } else if s == "MainContext" || s == "Main" {
        "Entity Context"
    } else {
        ""
    }
}

pub fn clean_text(s: &str) -> String {
    s.replace('Â', "")
        .replace('\u{00A0}', " ")
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}
