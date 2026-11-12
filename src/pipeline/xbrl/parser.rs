// src/pipeline/xbrl/parser.rs

use crate::domain::facts::{ParsedFiling, RawFact};
use crate::domain::units::{UnitRegistry, parse_number, parse_xbrl_numeric};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

type IntermediateFact = (String, String, Option<String>, Option<i16>, String);

fn is_valid_cin(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 21
        && matches!(b[0], b'L' | b'U')
        && b[1..6].iter().all(|c| c.is_ascii_digit())
        && b[6..8].iter().all(|c| c.is_ascii_uppercase())
        && b[8..12].iter().all(|c| c.is_ascii_digit())
        && b[12..15].iter().all(|c| c.is_ascii_uppercase())
        && b[15..21].iter().all(|c| c.is_ascii_digit())
}

fn decode_xml_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let u16s: Vec<u16> = bytes[2..]
            .as_chunks::<2>().0.iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&u16s);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let u16s: Vec<u16> = bytes[2..]
            .as_chunks::<2>().0.iter()
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&u16s);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn is_valid_xml_entity(remainder: &str) -> bool {
    if remainder.starts_with("amp;")
        || remainder.starts_with("lt;")
        || remainder.starts_with("gt;")
        || remainder.starts_with("apos;")
        || remainder.starts_with("quot;")
    {
        return true;
    }

    if let Some(rest) = remainder.strip_prefix('#')
        && let Some(semi_pos) = rest.find(';')
        && semi_pos > 0
        && semi_pos <= 8
    {
        let code = &rest[..semi_pos];
        if let Some(hex) = code.strip_prefix('x').or_else(|| code.strip_prefix('X')) {
            return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
        } else {
            return code.chars().all(|c| c.is_ascii_digit());
        }
    }

    false
}

fn needs_sanitization(raw: &str) -> bool {
    for (idx, ch) in raw.char_indices() {
        match ch as u32 {
            0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F..=0x84 | 0x86..=0x9F => return true,
            _ => {}
        }
        if ch == '&' && !is_valid_xml_entity(&raw[idx + 1..]) {
            return true;
        }
    }
    false
}

pub fn sanitize_xml_string(raw: &str) -> String {
    if !needs_sanitization(raw) {
        return raw.to_string();
    }

    let mut result = String::with_capacity(raw.len() + 256);
    for (idx, ch) in raw.char_indices() {
        let is_invalid = matches!(
            ch as u32,
            0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F..=0x84 | 0x86..=0x9F
        );

        if is_invalid {
            result.push(' ');
            continue;
        }

        if ch == '&' {
            if is_valid_xml_entity(&raw[idx + 1..]) {
                result.push('&');
            } else {
                result.push_str("&amp;");
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub struct XbrlParser<'a> {
    registry: &'a UnitRegistry,
}

impl<'a> XbrlParser<'a> {
    pub fn new(registry: &'a UnitRegistry) -> Self {
        Self { registry }
    }

    pub fn parse_file(
        &self,
        path: &Path,
    ) -> Result<ParsedFiling, Box<dyn std::error::Error + Send + Sync>> {
        let bytes = fs::read(path)?;
        let raw_content = match std::str::from_utf8(&bytes) {
            Ok(s) if !s.starts_with('\u{FEFF}') => s.to_string(),
            _ => decode_xml_bytes(&bytes),
        };
        drop(bytes);

        let sanitized = if needs_sanitization(&raw_content) {
            let s = sanitize_xml_string(&raw_content);
            drop(raw_content);
            s
        } else {
            raw_content
        };

        let mut reader = Reader::from_str(&sanitized);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut primary_cin = String::new();
        let mut fallback_cin = String::new();
        let mut detected_isin: Option<String> = None;
        let mut detected_symbol: Option<String> = None;
        let mut detected_scrip_code: Option<String> = None;
        let mut units_map: HashMap<String, String> = HashMap::new();

        let mut target_prefixes: HashSet<String> = HashSet::new();
        target_prefixes.insert("in-capmkt".to_string());
        target_prefixes.insert("incapmkt".to_string());
        target_prefixes.insert("in_capmkt".to_string());

        let mut intermediate_facts: Vec<IntermediateFact> = Vec::new();

        let mut current_unit_id = String::new();
        let mut in_measure = false;
        let mut in_divide = false;
        let mut in_numerator = false;
        let mut in_denominator = false;
        let mut numerator_measure = String::new();
        let mut denominator_measure = String::new();
        let mut current_measures = Vec::new();

        let mut in_context_id = String::new();
        let mut in_identifier = false;
        let mut identifier_scheme = String::new();
        let mut identifier_buf = String::new();

        let mut current_fact_tag = String::new();
        let mut current_context_ref = String::new();
        let mut current_unit_ref = None;
        let mut current_decimals = None;
        let mut current_fact_buf = String::new();
        let mut in_fact = false;

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let tag_str: &str = e.name().into_inner();

                    for attr in e.attributes().flatten() {
                        let k: &str = attr.key.into_inner();
                        let v: &str = attr.value.as_ref();

                        if (k.starts_with("xmlns:") || k == "xmlns")
                            && v.to_lowercase().contains("in-capmkt")
                        {
                            if let Some(prefix) = k.strip_prefix("xmlns:") {
                                target_prefixes.insert(prefix.to_string());
                            } else {
                                target_prefixes.insert(String::new());
                            }
                        }
                    }

                    if tag_str.ends_with(":context") || tag_str == "context" {
                        for attr in e.attributes().flatten() {
                            if attr.key.into_inner() == "id" {
                                in_context_id = attr.value.as_ref().to_string();
                            }
                        }
                    } else if tag_str.ends_with(":identifier") || tag_str == "identifier" {
                        in_identifier = true;
                        identifier_scheme.clear();
                        identifier_buf.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.into_inner() == "scheme" {
                                identifier_scheme = attr.value.as_ref().to_ascii_lowercase();
                            }
                        }
                    }

                    if tag_str.ends_with(":unit") || tag_str == "unit" {
                        for attr in e.attributes().flatten() {
                            if attr.key.into_inner() == "id" {
                                current_unit_id = attr.value.as_ref().to_string();
                            }
                        }
                        current_measures.clear();
                        in_divide = false;
                    } else if tag_str.ends_with(":divide") || tag_str == "divide" {
                        in_divide = true;
                        numerator_measure.clear();
                        denominator_measure.clear();
                    } else if tag_str.ends_with(":unitNumerator") || tag_str == "unitNumerator" {
                        in_numerator = true;
                    } else if tag_str.ends_with(":unitDenominator") || tag_str == "unitDenominator"
                    {
                        in_denominator = true;
                    } else if tag_str.ends_with(":measure") || tag_str == "measure" {
                        in_measure = true;
                    }

                    let (prefix, mut local_name) = match tag_str.split_once(':') {
                        Some((p, l)) => (p, l),
                        None => ("", tag_str),
                    };

                    if local_name.starts_with("in_capmkt_") {
                        local_name = local_name.trim_start_matches("in_capmkt_");
                    } else if local_name.starts_with("in-capmkt_") {
                        local_name = local_name.trim_start_matches("in-capmkt_");
                    }

                    let is_target_fact = target_prefixes.contains(prefix)
                        || tag_str.starts_with("in_capmkt")
                        || tag_str.starts_with("in-capmkt");

                    if is_target_fact && !in_fact {
                        let mut ctx_ref = String::new();
                        let mut u_ref = None;
                        let mut dec = None;

                        for attr in e.attributes().flatten() {
                            let k: &str = attr.key.into_inner();
                            let v: &str = attr.value.as_ref();
                            match k {
                                "contextRef" => ctx_ref = v.to_string(),
                                "unitRef" => u_ref = Some(v.to_string()),
                                "decimals" => dec = v.parse::<i16>().ok(),
                                _ => {}
                            }
                        }

                        if !ctx_ref.is_empty() {
                            current_fact_tag = local_name.to_string();
                            current_context_ref = ctx_ref;
                            current_unit_ref = u_ref;
                            current_decimals = dec;
                            current_fact_buf.clear();
                            in_fact = true;
                        }
                    }
                }

                Event::Text(ref e) => {
                    let text = quick_xml::escape::unescape(e.as_ref())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| e.as_ref().to_string());

                    if in_identifier {
                        identifier_buf.push_str(&text);
                    }

                    if in_measure && !current_unit_id.is_empty() {
                        let clean = text.trim();
                        if !clean.is_empty() {
                            let raw_measure = clean.split(':').next_back().unwrap_or(clean).trim();
                            // Preserve mL / ml distinction from ML (megaliters)
                            let measure = if raw_measure == "ml" || raw_measure == "mL" {
                                "MILLILITER".to_string()
                            } else {
                                raw_measure.to_uppercase()
                            };
                            if in_divide {
                                if in_numerator {
                                    numerator_measure = measure;
                                } else if in_denominator {
                                    denominator_measure = measure;
                                }
                            } else {
                                current_measures.push(measure);
                            }
                        }
                    }

                    if in_fact {
                        current_fact_buf.push_str(&text);
                    }
                }

                Event::CData(ref e) => {
                    if in_fact {
                        current_fact_buf.push_str(e.as_ref());
                    }
                }

                Event::End(ref e) => {
                    let tag_str: &str = e.name().into_inner();

                    if tag_str.ends_with(":context") || tag_str == "context" {
                        in_context_id.clear();
                    } else if tag_str.ends_with(":identifier") || tag_str == "identifier" {
                        in_identifier = false;
                        let clean_id = identifier_buf.trim().to_uppercase();
                        if !clean_id.is_empty()
                            && !matches!(
                                clean_id.as_str(),
                                "NA" | "N/A"
                                    | "NONE"
                                    | "NULL"
                                    | "NIL"
                                    | "-"
                                    | "DUMMY"
                                    | "UNKNOWN"
                                    | "ENTITY"
                                    | "COMPANY"
                                    | "TEST"
                                    | "000000"
                            )
                        {
                            if is_valid_cin(&clean_id) {
                                if fallback_cin.is_empty()
                                    || in_context_id.to_lowercase().contains("main")
                                    || in_context_id.to_lowercase().contains("dcy")
                                {
                                    fallback_cin = clean_id;
                                }
                            } else if clean_id.len() == 12
                                && clean_id.starts_with("IN")
                                && clean_id.bytes().all(|b| b.is_ascii_alphanumeric())
                            {
                                detected_isin.get_or_insert(clean_id);
                            } else if clean_id.len() == 6
                                && clean_id.bytes().all(|b| b.is_ascii_digit())
                                && (identifier_scheme.contains("bse")
                                    || identifier_scheme.contains("scrip"))
                            {
                                detected_scrip_code.get_or_insert(clean_id);
                            } else if clean_id.len() <= 20
                                && identifier_scheme.contains("nse")
                                && clean_id
                                    .bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'&')
                            {
                                detected_symbol.get_or_insert(clean_id);
                            }
                        }
                    } else if tag_str.ends_with(":measure") || tag_str == "measure" {
                        in_measure = false;
                    } else if tag_str.ends_with(":unit") || tag_str == "unit" {
                        if !current_unit_id.is_empty() {
                            let resolved = if in_divide
                                && !numerator_measure.is_empty()
                                && !denominator_measure.is_empty()
                            {
                                format!("{}/{}", numerator_measure, denominator_measure)
                            } else if !current_measures.is_empty() {
                                current_measures.join("/")
                            } else {
                                String::new()
                            };

                            units_map.insert(current_unit_id.clone(), resolved);
                        }
                        current_unit_id.clear();
                        in_divide = false;
                    } else if tag_str.ends_with(":unitNumerator") || tag_str == "unitNumerator" {
                        in_numerator = false;
                    } else if tag_str.ends_with(":unitDenominator") || tag_str == "unitDenominator"
                    {
                        in_denominator = false;
                    } else if in_fact {
                        let (_prefix, mut local_name) = match tag_str.split_once(':') {
                            Some((p, l)) => (p, l),
                            None => ("", tag_str),
                        };
                        if local_name.starts_with("in_capmkt_") {
                            local_name = local_name.trim_start_matches("in_capmkt_");
                        } else if local_name.starts_with("in-capmkt_") {
                            local_name = local_name.trim_start_matches("in-capmkt_");
                        }

                        if local_name == current_fact_tag {
                            let val = current_fact_buf.trim().to_string();
                            if !val.is_empty() {
                                if current_fact_tag == "CorporateIdentityNumber"
                                    && (primary_cin.is_empty()
                                        || current_context_ref.to_lowercase().contains("main")
                                        || current_context_ref.to_lowercase().contains("dcy"))
                                {
                                    primary_cin = val.trim().to_uppercase();
                                }

                                intermediate_facts.push((
                                    current_fact_tag.clone(),
                                    current_context_ref.clone(),
                                    current_unit_ref.clone(),
                                    current_decimals,
                                    val,
                                ));
                            }
                            in_fact = false;
                            current_fact_buf.clear();
                        }
                    }
                }

                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        let resolved_cin = if !primary_cin.is_empty() {
            primary_cin
        } else if !fallback_cin.is_empty() {
            fallback_cin
        } else {
            detected_isin
                .clone()
                .or_else(|| detected_symbol.clone())
                .or_else(|| detected_scrip_code.clone())
                .unwrap_or_default()
        };

        let mut final_facts = Vec::with_capacity(intermediate_facts.len());

        for (tag, ctx_ref, u_ref, dec, raw_val) in intermediate_facts {
            let std_unit_display = u_ref
                .as_ref()
                .and_then(|u| units_map.get(u).cloned().or_else(|| Some(u.clone())));

            let has_numeric_attrs = u_ref.is_some() || dec.is_some();
            let numeric_parsed = parse_xbrl_numeric(&raw_val, has_numeric_attrs);
            let datatype = if numeric_parsed.is_some() {
                "number"
            } else {
                "string"
            };

            let (std_val_str, converted_unit_str) = if let Some(ref unit_label) = std_unit_display {
                self.registry.convert_value(unit_label, &raw_val)
            } else if let Some(num) = numeric_parsed {
                (num.to_string(), String::new())
            } else {
                (raw_val.clone(), String::new())
            };

            let standard_val = if datatype == "number" {
                parse_number(&std_val_str).or(numeric_parsed)
            } else {
                None
            };
            let converted_unit = if converted_unit_str.is_empty() {
                None
            } else {
                Some(converted_unit_str)
            };

            final_facts.push(RawFact {
                client_id: resolved_cin.clone(),
                in_capmkt: tag,
                context_ref: ctx_ref,
                unit_ref: u_ref,
                decimals: dec,
                raw_value: raw_val,
                standard_unit_ref: std_unit_display,
                standard_value: standard_val,
                standard_value_str: if datatype == "number" {
                    Some(std_val_str)
                } else {
                    None
                },
                converted_unit,
                datatype,
            });
        }

        Ok(ParsedFiling {
            client_id: resolved_cin,
            detected_isin,
            detected_symbol,
            detected_scrip_code,
            facts: final_facts,
        })
    }
}
