// src/parser/units.rs

use regex::Regex;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static RE_ALPHANUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9]").unwrap());
static RE_KG_E: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^KG([A-Z0-9]+E)$").unwrap());

/// Zero-equivalent or non-applicable values
pub static ZEROISH: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("nil");
    s.insert("nill");
    s.insert("null");
    s.insert("none");
    s.insert("na");
    s.insert("n/a");
    s.insert("not applicable");
    s.insert("notapplicable");
    s.insert("not available");
    s.insert("notavailable");
    s.insert("-");
    s.insert("—");
    s.insert("–");
    s.insert("--");
    s
});

/// Parses numeric text strings, cleaning currency prefixes (₹, Rs), commas, percentage signs,
/// and accounting parentheses (e.g., "(1,000)" -> -1000.0).
pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }

    if ZEROISH.contains(t.to_lowercase().as_str()) {
        return Some(0.0);
    }

    let mut is_neg = false;
    let mut cleaned = t;

    // Accounting parentheses for negative numbers
    if cleaned.starts_with('(') && cleaned.ends_with(')') {
        is_neg = true;
        cleaned = &cleaned[1..cleaned.len() - 1];
    } else if cleaned.starts_with('-') {
        is_neg = true;
        cleaned = &cleaned[1..];
    }

    // Strip currency symbols, percentages, commas, and whitespace
    let normalized = cleaned
        .replace(',', "")
        .replace(' ', "")
        .replace('%', "")
        .replace('₹', "")
        .replace("Rs.", "")
        .replace("Rs", "")
        .replace('$', "");

    if let Ok(val) = normalized.parse::<f64>() {
        return Some(if is_neg { -val } else { val });
    }

    None
}

/// Normalizes raw unit strings (superscripts ², ³, µ, slashes)
pub fn normalize_unit_label(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let s2 = s
        .replace('µ', "u")
        .replace("Nm³", "NM3")
        .replace("Nm3", "NM3")
        .replace("m³", "m3")
        .replace("M³", "M3")
        .replace('/', "Per");

    RE_ALPHANUM.replace_all(&s2, "").to_uppercase()
}

#[derive(Debug, Clone)]
pub struct UnitRegistry {
    conversions: HashMap<String, (String, f64)>,
}

impl UnitRegistry {
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows =
            sqlx::query("SELECT raw_unit, canonical_unit, conversion_factor::double precision as conversion_factor FROM unit_conversions")
                .fetch_all(pool)
                .await?;

        let mut conversions = HashMap::with_capacity(rows.len());
        for row in rows {
            let raw: String = row.get("raw_unit");
            let canonical: String = row.get("canonical_unit");
            let factor: f64 = row.get("conversion_factor");
            conversions.insert(raw, (canonical, factor));
        }

        Ok(Self { conversions })
    }

    /// Standardizes units and computes mathematically converted values
    /// Returns: (standard_value_string, converted_unit_label)
    pub fn convert_value(&self, std_display_unit: &str, raw_val: &str) -> (String, String) {
        let v = match parse_number(raw_val) {
            Some(num) => num,
            None => return (raw_val.to_string(), String::new()),
        };

        let unit_key = normalize_unit_label(std_display_unit);

        if unit_key.is_empty() || unit_key == "PURE" {
            return (v.to_string(), String::new());
        }

        // Dynamic pollutant matching (e.g., KGNOxE -> GtNOxe)
        if unit_key.starts_with("KG")
            && unit_key.ends_with('E')
            && !self.conversions.contains_key(&unit_key)
        {
            if let Some(caps) = RE_KG_E.captures(&unit_key) {
                if let Some(m) = caps.get(1) {
                    let mut pol = m.as_str().to_string();
                    if pol.ends_with('E') {
                        pol.pop();
                        pol.push('e');
                    }
                    return ((v * 1e-12).to_string(), format!("Gt{}", pol));
                }
            }
        }

        if unit_key == "NM3" {
            return (v.to_string(), std_display_unit.to_string());
        }

        // Exact match or fallback without trailing 'S'
        let mut matched = self.conversions.get(&unit_key);
        if matched.is_none() && unit_key.ends_with('S') && unit_key != "PPM" {
            if let Some(stripped) = unit_key.strip_suffix('S') {
                matched = self.conversions.get(stripped);
            }
        }

        if let Some((to_unit, factor)) = matched {
            return ((v * factor).to_string(), to_unit.clone());
        }

        // Compound denominator ratios (e.g., MEGAJOULEPERINR -> GJ/INR)
        if unit_key.contains("PER") {
            let parts: Vec<&str> = unit_key.splitn(2, "PER").collect();
            let num = parts[0];
            let den = parts[1];

            if let Some((to_unit, factor)) = self.conversions.get(num) {
                return ((v * factor).to_string(), format!("{}/{}", to_unit, den));
            }
        }

        (v.to_string(), String::new())
    }
}
