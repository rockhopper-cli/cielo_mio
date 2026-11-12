// src/ui/views/form_viewer/tables/section_a.rs

use super::super::components::render_table_subheading;
use super::common::{CellData, fact_cell, find_fact, render_matrix_table, text_cell};
use crate::ui::state::{FormFactRow, FormMasterItem};
use eframe::egui::Ui;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn try_render(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) -> bool {
    let first = items[0];
    if first.section != "A" || first.principle_number != 0 {
        return false;
    }
    match first.question_number {
        16 => {
            render_assurance_scope_table(ui, items, exact_facts, salt);
            true
        }
        19 => {
            render_locations_table(ui, exact_facts, salt);
            true
        }
        21 => {
            render_employees_tables(ui, exact_facts, salt);
            true
        }
        22 => {
            render_women_representation_table(ui, exact_facts, salt);
            true
        }
        23 => {
            render_turnover_table(ui, exact_facts, salt);
            true
        }
        26 => {
            render_sec_a_complaints_table(ui, exact_facts, tag_facts, salt);
            true
        }
        _ => false,
    }
}

fn render_assurance_scope_table(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let mut groups: BTreeMap<&str, [Option<&FormMasterItem>; 4]> = BTreeMap::new();
    for item in items {
        let idx = match item.question_order {
            1..=4 => (item.question_order - 1) as usize,
            _ => continue,
        };
        groups
            .entry(item.followup_question_first.as_str())
            .or_insert([None, None, None, None])[idx] = Some(item);
    }

    let mut rows = Vec::new();
    for (_fu, cols) in groups {
        let Some(t_item) = cols[0].or(cols[1]) else {
            continue;
        };
        let assured_f = cols[1].and_then(|m| {
            find_fact(
                exact_facts,
                m.standard_indicator_capmkt.trim(),
                m.standard_context_reference.trim(),
            )
        });
        let scope_f = cols[2].and_then(|m| {
            find_fact(
                exact_facts,
                m.standard_indicator_capmkt.trim(),
                m.standard_context_reference.trim(),
            )
        });
        let rem_f = cols[3].and_then(|m| {
            find_fact(
                exact_facts,
                m.standard_indicator_capmkt.trim(),
                m.standard_context_reference.trim(),
            )
        });

        if assured_f.is_none() && scope_f.is_none() && rem_f.is_none() {
            continue;
        }
        rows.push(vec![
            text_cell(&t_item.question_text),
            fact_cell(assured_f, "boolean"),
            fact_cell(scope_f, "string"),
            fact_cell(rem_f, "string"),
        ]);
    }

    let avail = ui.available_width().max(460.0);
    let c0 = (avail * 0.40).clamp(180.0, 320.0);
    let c1 = 85.0;
    let c2 = 120.0;
    let c3 = (avail - c0 - c1 - c2 - 20.0).max(120.0);
    render_matrix_table(
        ui,
        (salt, "assurance_scope"),
        &[
            "BRSR Section / Question",
            "Assured?",
            "Assurance Type",
            "Remarks",
        ],
        &[c0, c1, c2, c3],
        &rows,
    );
}

fn render_locations_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let rows = vec![
        vec![
            text_cell("National"),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Plant_National"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Office_National"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Location_National"),
                "number",
            ),
        ],
        vec![
            text_cell("International"),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Plant_International"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Office_International"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "NumberOfLocations", "D_Location_International"),
                "number",
            ),
        ],
    ];
    let avail = ui.available_width().max(420.0);
    let c0 = (avail * 0.28).clamp(110.0, 180.0);
    let cw = ((avail - c0 - 20.0) / 3.0).max(90.0);
    render_matrix_table(
        ui,
        (salt, "locations"),
        &["Location", "Number of plants", "Number of offices", "Total"],
        &[c0, cw, cw, cw],
        &rows,
    );
}

fn render_employees_tables(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let headers = &[
        "Particulars",
        "Total (A)",
        "Male No. (B)",
        "Male % (B/A)",
        "Female No. (C)",
        "Female % (C/A)",
        "Other No. (H)",
        "Other % (H/A)",
    ];
    let widths = &[195.0, 82.0, 82.0, 82.0, 82.0, 82.0, 82.0, 82.0];

    let build_emp_rows =
        |suffix: &str, count_tag: &str, pct_tag: &str| -> Vec<Vec<Option<CellData>>> {
            let cats = [
                ("Permanent Employees (D)", "PermanentEmployees"),
                (
                    "Other than Permanent Employees (E)",
                    "OtherThanPermanentEmployees",
                ),
                ("Total Employees (D + E)", "Employees"),
                ("Permanent Workers (F)", "PermanentWorkers"),
                (
                    "Other than Permanent Workers (G)",
                    "OtherThanPermanentWorkers",
                ),
                ("Total Workers (F + G)", "Workers"),
            ];
            cats.iter()
                .map(|(label, seg)| {
                    vec![
                        text_cell(label),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                count_tag,
                                &format!("D_Gender_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                count_tag,
                                &format!("D_Male_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                        fact_cell(
                            find_fact(exact_facts, pct_tag, &format!("D_Male_{}_{}", seg, suffix)),
                            "number",
                        ),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                count_tag,
                                &format!("D_Female_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                pct_tag,
                                &format!("D_Female_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                count_tag,
                                &format!("D_OtherGender_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                        fact_cell(
                            find_fact(
                                exact_facts,
                                pct_tag,
                                &format!("D_OtherGender_{}_{}", seg, suffix),
                            ),
                            "number",
                        ),
                    ]
                })
                .collect()
        };

    render_table_subheading(ui, "a. Employees and workers (including differently abled)");
    let rows_a = build_emp_rows(
        "TableA",
        "NumberOfEmployeesOrWorkersIncludingDifferentlyAbled",
        "PercentageOfEmployeesOrWorkersIncludingDifferentlyAbled",
    );
    render_matrix_table(ui, (salt, "emp_a"), headers, widths, &rows_a);

    ui.add_space(6.0);
    render_table_subheading(ui, "b. Differently abled Employees and workers");
    let rows_b = build_emp_rows(
        "TableB",
        "NumberOfDifferentlyAbledEmployeesOrWorkers",
        "PercentageOfDifferentlyAbledEmployeesOrWorkers",
    );
    render_matrix_table(ui, (salt, "emp_b"), headers, widths, &rows_b);
}

fn render_women_representation_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let rows = vec![
        vec![
            text_cell("Board of Directors"),
            fact_cell(
                find_fact(exact_facts, "TotalNumberOfBoardOfDirectors", "DCYMain"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "NumberOfFemaleBoardOfDirectors", "DCYMain"),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "PercentageOfFemaleBoardOfDirectors", "DCYMain"),
                "number",
            ),
        ],
        vec![
            text_cell("Key Management Personnel"),
            fact_cell(
                find_fact(
                    exact_facts,
                    "TotalNumberOfKeyManagementPersonnel",
                    "DCYMain",
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfFemaleKeyManagementPersonnel",
                    "DCYMain",
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "PercentageOfFemaleKeyManagementPersonnel",
                    "DCYMain",
                ),
                "number",
            ),
        ],
    ];
    let avail = ui.available_width().max(420.0);
    let c0 = (avail * 0.34).clamp(150.0, 240.0);
    let cw = ((avail - c0 - 20.0) / 3.0).max(85.0);
    render_matrix_table(
        ui,
        (salt, "women_rep"),
        &["Category", "Total (A)", "No. of Females (B)", "% (B/A)"],
        &[c0, cw, cw, cw],
        &rows,
    );
}

fn render_turnover_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let mut rows = Vec::new();
    for (cat_label, seg) in [
        ("Permanent Employees", "PermanentEmployees"),
        ("Permanent Workers", "PermanentWorkers"),
    ] {
        for (g_label, g_ctx) in [
            ("Male", "D_Male"),
            ("Female", "D_Female"),
            ("Other Gender", "D_OtherGender"),
            ("Total", "D_Gender"),
        ] {
            rows.push(vec![
                text_cell(&format!("{} — {}", cat_label, g_label)),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "TurnoverRate",
                        &format!("{}_{}_TableB_TurnOverRate_CY", g_ctx, seg),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "TurnoverRate",
                        &format!("{}_{}_TableB_TurnOverRate_PY", g_ctx, seg),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "TurnoverRate",
                        &format!("{}_{}_TableB_TurnOverRate_PPY", g_ctx, seg),
                    ),
                    "number",
                ),
            ]);
        }
    }
    let avail = ui.available_width().max(420.0);
    let c0 = (avail * 0.38).clamp(170.0, 260.0);
    let cw = ((avail - c0 - 20.0) / 3.0).max(85.0);
    render_matrix_table(
        ui,
        (salt, "turnover"),
        &[
            "Category / Gender",
            "Current FY (%)",
            "Previous FY (%)",
            "Prior to Previous FY (%)",
        ],
        &[c0, cw, cw, cw],
        &rows,
    );
}

fn render_sec_a_complaints_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let fixed = [
        (
            "Communities",
            "D_ComplaintReceivedFromCommunities",
            "I_ComplaintReceivedFromCommunities",
        ),
        (
            "Investors (other than shareholders)",
            "D_ComplaintReceivedFromInvestors",
            "I_ComplaintReceivedFromInvestors",
        ),
        (
            "Shareholders",
            "D_ComplaintReceivedFromShareholders",
            "I_ComplaintReceivedFromShareholders",
        ),
        (
            "Employees and workers",
            "D_ComplaintReceivedFromEmployeesAndWorkers",
            "I_ComplaintReceivedFromEmployeesAndWorkers",
        ),
        (
            "Customers",
            "D_ComplaintReceivedFromCustomers",
            "I_ComplaintReceivedFromCustomers",
        ),
        (
            "Value Chain Partners",
            "D_ComplaintReceivedFromValueChainPartners",
            "I_ComplaintReceivedFromValueChainPartners",
        ),
    ];

    let mut rows: Vec<Vec<Option<CellData>>> = fixed
        .iter()
        .map(|(label, d_ctx, i_ctx)| {
            vec![
                text_cell(label),
                fact_cell(find_fact(exact_facts, "GrievanceRedressalMechanismInPlace", d_ctx), "boolean"),
                fact_cell(
                    find_fact(exact_facts, "WebLinkForGrievanceRedressPolicy", d_ctx)
                        .or_else(|| find_fact(exact_facts, "DetailsForGrievanceRedressalMechanismInPlaceIsNotApplicableExplanatoryTextBlock", d_ctx)),
                    "hyperlink",
                ),
                fact_cell(find_fact(exact_facts, "NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear", d_ctx), "number"),
                fact_cell(find_fact(exact_facts, "NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear", i_ctx), "number"),
                fact_cell(find_fact(exact_facts, "RemarksStakeHolderGroupFromWhomComplaintIsReceived", d_ctx), "string"),
                fact_cell(find_fact(exact_facts, "NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear", &format!("{}_PY", d_ctx)), "number"),
                fact_cell(find_fact(exact_facts, "NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear", &format!("{}_PY", i_ctx)), "number"),
                fact_cell(find_fact(exact_facts, "RemarksStakeHolderGroupFromWhomComplaintIsReceived", &format!("{}_PY", d_ctx)), "string"),
            ]
        })
        .collect();

    let mut other_indices = BTreeSet::new();
    if let Some(list) = tag_facts.get("GrievanceRedressalMechanismInPlace") {
        for f in list {
            if let Some(idx) = f.context_ref.strip_prefix("D_OtherStakeHolder")
                && !idx.is_empty()
                && idx.chars().all(|c| c.is_ascii_digit())
            {
                other_indices.insert(idx.to_string());
            }
        }
    }
    for idx in other_indices {
        let d_ctx = format!("D_OtherStakeHolder{}", idx);
        let i_ctx = format!("I_OtherStakeHolder{}", idx);
        let d_py = format!("D_OtherStakeHolder_PY{}", idx);
        let i_py = format!("I_OtherStakeHolder_PY{}", idx);
        let name = find_fact(exact_facts, "NameOfStakeHolderGroup", &i_ctx)
            .map(|f| f.raw_value.clone())
            .unwrap_or_else(|| format!("Other Stakeholder {}", idx));

        rows.push(vec![
            text_cell(&name),
            fact_cell(
                find_fact(exact_facts, "GrievanceRedressalMechanismInPlace", &d_ctx),
                "boolean",
            ),
            fact_cell(
                find_fact(exact_facts, "WebLinkForGrievanceRedressPolicy", &d_ctx),
                "hyperlink",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear",
                    &d_ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear",
                    &i_ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "RemarksStakeHolderGroupFromWhomComplaintIsReceived",
                    &d_ctx,
                ),
                "string",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfComplaintsFiledFromStakeHolderGroupDuringTheYear",
                    &d_py,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfComplaintsPendingFromStakeHolderGroupResolutionAtTheEndOfYear",
                    &i_py,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "RemarksStakeHolderGroupFromWhomComplaintIsReceived",
                    &d_py,
                ),
                "string",
            ),
        ]);
    }

    render_matrix_table(
        ui,
        (salt, "sec_a_complaints"),
        &[
            "Stakeholder Group",
            "Mechanism?",
            "Policy Web-Link / NA Reason",
            "CY Filed",
            "CY Pending",
            "CY Remarks",
            "PY Filed",
            "PY Pending",
            "PY Remarks",
        ],
        &[160.0, 82.0, 150.0, 75.0, 80.0, 130.0, 75.0, 80.0, 130.0],
        &rows,
    );
}
