// src/parser/xml.rs

use crate::parser::units::{UnitRegistry, parse_number};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

fn is_valid_xml_entity(remainder: &str) -> bool {
    if remainder.starts_with("amp;")
        || remainder.starts_with("lt;")
        || remainder.starts_with("gt;")
        || remainder.starts_with("apos;")
        || remainder.starts_with("quot;")
    {
        return true;
    }

    if let Some(rest) = remainder.strip_prefix('#') {
        if let Some(semi_pos) = rest.find(';') {
            if semi_pos > 0 && semi_pos <= 8 {
                let code = &rest[..semi_pos];
                if let Some(hex) = code.strip_prefix('x').or_else(|| code.strip_prefix('X')) {
                    return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
                } else {
                    return code.chars().all(|c| c.is_ascii_digit());
                }
            }
        }
    }

    false
}

pub fn sanitize_xml_string(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        let is_invalid = match ch as u32 {
            0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F..=0x84 | 0x86..=0x9F => true,
            _ => false,
        };

        if is_invalid {
            result.push(' ');
            continue;
        }

        if ch == '&' {
            let remainder = &raw[idx + 1..];
            if is_valid_xml_entity(remainder) {
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

#[derive(Debug, Clone)]
pub struct RawFact {
    pub client_id: String,
    pub in_capmkt: String,
    pub context_ref: String,
    pub unit_ref: Option<String>,
    pub decimals: Option<i16>,
    pub raw_value: String,
    pub standard_unit_ref: Option<String>,
    pub standard_value: Option<f64>,
    pub converted_unit: Option<String>,
    pub datatype: &'static str,
}

#[derive(Debug, Clone)]
pub struct ParsedFiling {
    pub client_id: String,
    pub facts: Vec<RawFact>,
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
        let raw_bytes = fs::read(path)?;
        let raw_str = String::from_utf8_lossy(&raw_bytes);
        let sanitized = sanitize_xml_string(&raw_str);

        let mut reader = Reader::from_str(&sanitized);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut primary_cin = String::new();
        let mut fallback_cin = String::new();
        let mut units_map: HashMap<String, String> = HashMap::new();

        let mut target_prefixes: HashSet<String> = HashSet::new();
        target_prefixes.insert("in-capmkt".to_string());
        target_prefixes.insert("incapmkt".to_string());
        target_prefixes.insert("in_capmkt".to_string());

        let mut intermediate_facts: Vec<(String, String, Option<String>, Option<i16>, String)> =
            Vec::new();

        // State trackers for nested units
        let mut current_unit_id = String::new();
        let mut in_divide = false;
        let mut in_numerator = false;
        let mut in_denominator = false;
        let mut numerator_measure = String::new();
        let mut denominator_measure = String::new();
        let mut current_measures = Vec::new();

        // State trackers for context identifiers
        let mut in_context_id = String::new();
        let mut in_identifier = false;
        let mut identifier_buf = String::new();

        // Current fact accumulator
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

                    // Register any namespace matching SEBI in-capmkt
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

                    // Context tracking for identifier fallback
                    if tag_str.ends_with(":context") || tag_str == "context" {
                        for attr in e.attributes().flatten() {
                            if attr.key.into_inner() == "id" {
                                in_context_id = attr.value.as_ref().to_string();
                            }
                        }
                    } else if tag_str.ends_with(":identifier") || tag_str == "identifier" {
                        in_identifier = true;
                        identifier_buf.clear();
                    }

                    // Unit element tracking
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
                    }

                    // Fact element detection: matches prefixes or in-capmkt tag patterns
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

                    if is_target_fact {
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

                // Handle text chunks without dropping multi-part nodes
                Event::Text(ref e) => {
                    let text = quick_xml::escape::unescape(e.as_ref())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| e.as_ref().to_string());

                    if in_identifier {
                        identifier_buf.push_str(&text);
                    }

                    if !current_unit_id.is_empty() {
                        let clean = text.trim();
                        if !clean.is_empty() {
                            let measure = clean.split(':').last().unwrap_or(clean).to_uppercase();
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

                // Preserve CDATA disclosure blocks
                Event::CData(ref e) => {
                    let cdata_str = e.as_ref();
                    if in_fact {
                        current_fact_buf.push_str(cdata_str);
                    }
                }

                Event::End(ref e) => {
                    let tag_str: &str = e.name().into_inner();

                    if tag_str.ends_with(":context") || tag_str == "context" {
                        in_context_id.clear();
                    } else if tag_str.ends_with(":identifier") || tag_str == "identifier" {
                        in_identifier = false;
                        let clean_id = identifier_buf.trim().to_string();
                        if !clean_id.is_empty()
                            && (fallback_cin.is_empty()
                                || in_context_id.to_lowercase().contains("main")
                                || in_context_id.to_lowercase().contains("dcy"))
                        {
                            fallback_cin = clean_id;
                        }
                    } else if tag_str.ends_with(":unit") || tag_str == "unit" {
                        if !current_unit_id.is_empty() {
                            let resolved = if in_divide
                                && !numerator_measure.is_empty()
                                && !denominator_measure.is_empty()
                            {
                                format!("{}Per{}", numerator_measure, denominator_measure)
                            } else if !current_measures.is_empty() {
                                current_measures.join("Per")
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
                        let (prefix, mut local_name) = match tag_str.split_once(':') {
                            Some((p, l)) => (p, l),
                            None => ("", tag_str),
                        };
                        if local_name.starts_with("in_capmkt_") {
                            local_name = local_name.trim_start_matches("in_capmkt_");
                        } else if local_name.starts_with("in-capmkt_") {
                            local_name = local_name.trim_start_matches("in-capmkt_");
                        }

                        if local_name == current_fact_tag || target_prefixes.contains(prefix) {
                            let val = current_fact_buf.trim().to_string();
                            if !val.is_empty() {
                                if current_fact_tag == "CorporateIdentityNumber" {
                                    if primary_cin.is_empty()
                                        || current_context_ref.to_lowercase().contains("main")
                                        || current_context_ref.to_lowercase().contains("dcy")
                                    {
                                        primary_cin = val.clone();
                                    }
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
        } else {
            fallback_cin
        };

        let mut final_facts = Vec::with_capacity(intermediate_facts.len());

        for (tag, ctx_ref, u_ref, dec, raw_val) in intermediate_facts {
            let std_unit_display = u_ref
                .as_ref()
                .and_then(|u| units_map.get(u).cloned().or_else(|| Some(u.clone())));

            let is_numeric = parse_number(&raw_val).is_some();
            let datatype = if is_numeric { "number" } else { "string" };

            let (std_val_str, converted_unit_str) = if let Some(ref unit_label) = std_unit_display {
                self.registry.convert_value(unit_label, &raw_val)
            } else {
                (raw_val.clone(), String::new())
            };

            let standard_val = parse_number(&std_val_str);
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
                converted_unit,
                datatype,
            });
        }

        Ok(ParsedFiling {
            client_id: resolved_cin,
            facts: final_facts,
        })
    }
}
