// src/domain/units.rs

use core::str::FromStr;
use regex::Regex;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static RE_ALPHANUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9]").unwrap());
static RE_KG_E: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^KG([A-Z0-9]+E)$").unwrap());

/// Zero-equivalent or non-applicable strings used by visibility gates and placeholder checks
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

fn clean_numeric_str(s: &str) -> Option<(bool, String)> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }

    let mut is_neg = false;
    let mut cleaned = t;

    if cleaned.starts_with('(') && cleaned.ends_with(')') && cleaned.len() > 2 {
        is_neg = true;
        cleaned = cleaned[1..cleaned.len() - 1].trim();
    }

    if let Some(rest) = cleaned.strip_prefix('-') {
        is_neg = !is_neg;
        cleaned = rest.trim();
    } else if let Some(rest) = cleaned.strip_prefix('+') {
        cleaned = rest.trim();
    }

    let mut normalized = cleaned.replace([',', ' ', '\u{00A0}', '\t', '%', '₹', '$'], "");
    for prefix in ["Rs.", "rs.", "RS.", "Rs", "rs", "RS", "INR", "inr"] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            normalized = rest.to_string();
            break;
        }
    }

    if normalized.is_empty() {
        return None;
    }

    Some((is_neg, normalized))
}

/// Exact base-10 numeric parser avoiding binary f64 rounding artifacts
pub fn parse_decimal(s: &str) -> Option<Decimal> {
    let (is_neg, normalized) = clean_numeric_str(s)?;
    let parsed = Decimal::from_str(&normalized)
        .or_else(|_| Decimal::from_scientific(&normalized))
        .ok()?;
    Some(if is_neg { -parsed } else { parsed })
}

/// Parses real numeric text strings into f64 (used by visibility numeric thresholds)
pub fn parse_number(s: &str) -> Option<f64> {
    if let Some(d) = parse_decimal(s)
        && let Some(f) = d.to_f64()
        && f.is_finite()
    {
        return Some(f);
    }
    let (is_neg, normalized) = clean_numeric_str(s)?;
    if let Ok(val) = normalized.parse::<f64>()
        && val.is_finite()
    {
        return Some(if is_neg { -val } else { val });
    }
    None
}

/// Parses numeric XBRL element text into exact Decimal where explicit "nil"/"nill" represents 0
pub fn parse_xbrl_decimal(s: &str, has_numeric_attrs: bool) -> Option<Decimal> {
    if let Some(d) = parse_decimal(s) {
        return Some(d);
    }
    if has_numeric_attrs {
        let lo = s.trim().to_ascii_lowercase();
        if lo == "nil" || lo == "nill" {
            return Some(Decimal::ZERO);
        }
    }
    None
}

/// Normalizes a single unit segment while preserving case-sensitive `mL`/`ml` vs `ML`
fn normalize_unit_segment(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed == "ml" || trimmed == "mL" {
        return "MILLILITER".to_string();
    }
    let s2 = trimmed
        .replace('µ', "u")
        .replace("Nm³", "NM3")
        .replace("Nm3", "NM3")
        .replace("m³", "m3")
        .replace("M³", "M3");
    RE_ALPHANUM.replace_all(&s2, "").to_uppercase()
}

/// Normalizes raw unit strings (superscripts ², ³, µ, slashes)
pub fn normalize_unit_label(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if let Some((num, den)) = s.split_once('/') {
        let n = normalize_unit_segment(num);
        let d = normalize_unit_segment(den);
        if d.is_empty() {
            n
        } else {
            format!("{}/{}", n, d)
        }
    } else if let Some((num, den)) = s.split_once("Per")
        && !num.trim().is_empty()
    {
        // Only treat "Per" as ratio separator when preceded by a numerator (guards "Person"/"Percentage")
        let n = normalize_unit_segment(num);
        let d = normalize_unit_segment(den);
        if d.is_empty() {
            n
        } else {
            format!("{}/{}", n, d)
        }
    } else {
        normalize_unit_segment(s)
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnitRegistry {
    conversions: HashMap<String, (String, Decimal)>,
}

impl UnitRegistry {
    /// Loads conversions from database via repository
    pub async fn load_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let rows = crate::storage::repository::taxonomy::fetch_unit_conversions(pool).await?;
        let mut conversions = HashMap::with_capacity(rows.len());
        for (raw, canonical, factor) in rows {
            conversions.insert(raw, (canonical, factor));
        }
        Ok(Self { conversions })
    }

    fn lookup_unit(&self, key: &str) -> Option<&(String, Decimal)> {
        if let Some(hit) = self.conversions.get(key) {
            return Some(hit);
        }
        if key.ends_with('S')
            && key != "PPM"
            && key != "LBS"
            && let Some(stripped) = key.strip_suffix('S')
        {
            return self.conversions.get(stripped);
        }
        None
    }

    /// Standardizes units and computes exact base-10 converted values using Decimal
    /// Returns: (converted_decimal_opt, standard_value_string, converted_unit_label)
    pub fn convert_decimal(
        &self,
        std_display_unit: &str,
        raw_val: &str,
    ) -> (Option<Decimal>, String, String) {
        let v_dec = match parse_decimal(raw_val) {
            Some(d) => d,
            None => return (None, raw_val.to_string(), String::new()),
        };

        let unit_key = normalize_unit_label(std_display_unit);
        if unit_key.is_empty() || unit_key == "PURE" {
            let norm = v_dec.normalize();
            return (Some(norm), norm.to_string(), String::new());
        }

        // Dynamic pollutant matching (e.g., KGNOXE -> tNOxe)
        if !unit_key.contains('/')
            && unit_key.starts_with("KG")
            && unit_key.ends_with('E')
            && self.lookup_unit(&unit_key).is_none()
            && let Some(caps) = RE_KG_E.captures(&unit_key)
            && let Some(m) = caps.get(1)
        {
            let mut pol = m.as_str().to_string();
            if pol.ends_with('E') {
                pol.pop();
                pol.push('e');
            }
            let converted = v_dec
                .checked_mul(Decimal::new(1, 3))
                .unwrap_or(v_dec)
                .normalize();
            return (Some(converted), converted.to_string(), format!("t{}", pol));
        }

        if unit_key == "NM3" {
            let norm = v_dec.normalize();
            return (Some(norm), norm.to_string(), std_display_unit.to_string());
        }

        if let Some((to_unit, factor)) = self.lookup_unit(&unit_key) {
            let converted = v_dec.checked_mul(*factor).unwrap_or(v_dec).normalize();
            return (Some(converted), converted.to_string(), to_unit.clone());
        }

        // Explicit ratio conversion
        if let Some((num, den)) = unit_key.split_once('/')
            && let Some((to_unit, factor)) = self.lookup_unit(num)
        {
            let converted = v_dec.checked_mul(*factor).unwrap_or(v_dec).normalize();
            return (
                Some(converted),
                converted.to_string(),
                format!("{}/{}", to_unit, den),
            );
        }

        let norm = v_dec.normalize();
        (Some(norm), norm.to_string(), String::new())
    }
}
