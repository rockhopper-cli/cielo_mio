// src/ui/views/form_viewer/visibility.rs

use super::profile::resolve_profile_value;
use crate::domain::units::{ZEROISH, parse_number};
use crate::ui::state::{CompanyFilingSummary, FormFactRow, FormMasterItem};
use std::collections::HashMap;

pub type QuestionGroupKey<'a> = (&'a str, i16, i16, i16);
pub type QuestionGateRule<'a> = (&'a str, Option<&'a str>);
pub type QuestionGatesMap<'a> = HashMap<QuestionGroupKey<'a>, QuestionGateRule<'a>>;

/// Matches facts by `(std_tag, std_ctx)` first, only falling back to tag-level matching when
/// the template context is empty or generic CYMain (`DCYMain`/`ICYMain`)—never leaking CY into PY/dimensional cells.
pub(super) fn lookup_item_facts<'a>(
    item: &FormMasterItem,
    exact_facts: &HashMap<(&str, &str), Vec<&'a FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&'a FormFactRow>>,
) -> Vec<&'a FormFactRow> {
    let std_ind = item.standard_indicator_capmkt.trim();
    let std_ctx = item.standard_context_reference.trim();

    if let Some(v) = exact_facts.get(&(std_ind, std_ctx)) {
        return v.clone();
    }

    if !std_ind.is_empty()
        && (std_ctx.is_empty() || std_ctx == "DCYMain" || std_ctx == "ICYMain")
        && let Some(list) = tag_facts.get(std_ind)
    {
        if std_ctx.is_empty() {
            return list.clone();
        }
        return list
            .iter()
            .copied()
            .filter(|f| {
                let c = f.context_ref.to_ascii_lowercase();
                !c.contains("py") && !c.contains("previous") && !c.contains("prior")
            })
            .collect();
    }

    Vec::new()
}

/// Pre-indexes all reported responses by question_key across the active filing
pub fn build_response_index<'a>(
    form_template: &'a [FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    current_profile: Option<&CompanyFilingSummary>,
) -> HashMap<&'a str, String> {
    let mut responses = HashMap::with_capacity(form_template.len());

    for item in form_template {
        let facts = lookup_item_facts(item, exact_facts, tag_facts);
        let fact_val = facts.first().map(|f| f.raw_value.clone());

        let final_val = fact_val
            .or_else(|| resolve_profile_value(item, current_profile).map(|s| s.into_owned()));

        if let Some(val) = final_val {
            responses.insert(item.question_key.trim(), val);
        }
    }

    responses
}

/// Only treats an item's gate as a question-wide gate when the first item in the question
/// (or a non-sub-branching question gate) declares it—never letting an "If Yes/NA" child row
/// lock out unrelated sibling rows in the same question.
pub fn build_question_gates<'a>(form_template: &'a [FormMasterItem]) -> QuestionGatesMap<'a> {
    let mut gates = HashMap::new();
    for item in form_template {
        let dep = item.visibility_dependence.trim();
        if !dep.is_empty() && dep != "0" && !dep.eq_ignore_ascii_case("none") {
            let trig = item.visibility_trigger.as_deref().unwrap_or("").trim();
            // Do not hoist per-row "If Yes / If No / If NA" detail gates onto the whole question
            let is_local_branch = item.section == "A" && item.question_number == 26;
            if is_local_branch {
                continue;
            }
            if item.question_order <= 2 || trig.eq_ignore_ascii_case("all") {
                let key: QuestionGroupKey<'a> = (
                    item.section.as_str(),
                    item.principle_number,
                    item.sub_section,
                    item.question_number,
                );
                let rule: QuestionGateRule<'a> = (dep, item.visibility_trigger.as_deref());
                gates.entry(key).or_insert(rule);
            }
        }
    }
    gates
}

fn has_reported_entry(item_key: &str, reported_responses: &HashMap<&str, String>) -> bool {
    if let Some(val) = reported_responses.get(item_key) {
        let v = val.trim();
        let v_lower = v.to_lowercase();
        !v.is_empty()
            && !ZEROISH.contains(v_lower.as_str())
            && !v.eq_ignore_ascii_case("no")
            && !v.eq_ignore_ascii_case("false")
            && !v.eq_ignore_ascii_case("0")
            && !v.eq_ignore_ascii_case("— not disclosed")
            && !v.eq_ignore_ascii_case("not disclosed")
    } else {
        false
    }
}

/// Evaluates whether a question satisfies its statutory visibility trigger.
/// Always stays visible if the filing itself explicitly reported a fact for this item.
pub fn is_visibility_satisfied(
    item: &FormMasterItem,
    question_gates: &QuestionGatesMap<'_>,
    reported_responses: &HashMap<&str, String>,
) -> bool {
    // If the company actually filed a value for this specific item, never hide real data
    if reported_responses.contains_key(item.question_key.trim()) {
        return true;
    }

    let own_dep = item.visibility_dependence.trim();
    let (dep, trig_opt) =
        if !own_dep.is_empty() && own_dep != "0" && !own_dep.eq_ignore_ascii_case("none") {
            (own_dep, item.visibility_trigger.as_deref())
        } else {
            let q_key: QuestionGroupKey<'_> = (
                item.section.as_str(),
                item.principle_number,
                item.sub_section,
                item.question_number,
            );
            match question_gates.get(&q_key) {
                Some(&(parent_dep, parent_trig)) => (parent_dep, parent_trig),
                None => ("0", None),
            }
        };

    if dep.is_empty() || dep == "0" || dep.eq_ignore_ascii_case("none") {
        return true;
    }

    let trig = match trig_opt {
        Some(t) if !t.trim().is_empty() && t.trim() != "0" => t.trim(),
        _ => return true,
    };

    if let Some(reported_raw) = reported_responses.get(dep) {
        let resp = reported_raw.trim();
        let resp_lower = resp.to_lowercase();

        if trig.eq_ignore_ascii_case("anyvalue") {
            return !resp.is_empty()
                && !ZEROISH.contains(resp_lower.as_str())
                && !resp.eq_ignore_ascii_case("— not disclosed")
                && !resp.eq_ignore_ascii_case("not disclosed");
        }

        if let Some(rest) = trig.strip_prefix("<=") {
            if let (Some(num), Ok(target)) = (parse_number(resp), rest.trim().parse::<f64>()) {
                return num <= target;
            }
        } else if let Some(rest) = trig.strip_prefix(">=") {
            if let (Some(num), Ok(target)) = (parse_number(resp), rest.trim().parse::<f64>()) {
                return num >= target;
            }
        } else if let Some(rest) = trig.strip_prefix('<') {
            if let (Some(num), Ok(target)) = (parse_number(resp), rest.trim().parse::<f64>()) {
                return num < target;
            }
        } else if let Some(rest) = trig.strip_prefix('>')
            && let (Some(num), Ok(target)) = (parse_number(resp), rest.trim().parse::<f64>())
        {
            return num > target;
        }

        if trig.eq_ignore_ascii_case("all") {
            let is_all = resp.eq_ignore_ascii_case("all") || resp_lower.starts_with("all ");
            let is_partial = resp.eq_ignore_ascii_case("partial") || resp_lower.contains("partial");

            if !is_all && !is_partial {
                return false;
            }

            if item.question_type.eq_ignore_ascii_case("detail") {
                return has_reported_entry(item.question_key.trim(), reported_responses);
            }

            return is_all || has_reported_entry(item.question_key.trim(), reported_responses);
        }

        if trig.eq_ignore_ascii_case("yes") || trig.eq_ignore_ascii_case("true") || trig == "1" {
            return resp.eq_ignore_ascii_case("yes")
                || resp.eq_ignore_ascii_case("true")
                || resp == "1";
        }
        if trig.eq_ignore_ascii_case("no") || trig.eq_ignore_ascii_case("false") || trig == "0" {
            return resp.eq_ignore_ascii_case("no")
                || resp.eq_ignore_ascii_case("false")
                || resp == "0";
        }

        if trig.to_lowercase().starts_with("other") {
            return resp_lower.contains("other");
        }

        if trig.eq_ignore_ascii_case("na") || trig.eq_ignore_ascii_case("not applicable") {
            return resp.eq_ignore_ascii_case("na")
                || resp.eq_ignore_ascii_case("n/a")
                || resp.eq_ignore_ascii_case("not applicable");
        }

        resp.eq_ignore_ascii_case(trig) || resp_lower.contains(&trig.to_lowercase())
    } else {
        false
    }
}
