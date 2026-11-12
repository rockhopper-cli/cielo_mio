// src/ui/views/form_viewer/tables/section_c.rs

use super::super::components::render_table_subheading;
use super::common::{CellData, fact_cell, find_fact, render_matrix_table, text_cell};
use super::generic::render_cy_py_table;
use crate::ui::state::{FormFactRow, FormMasterItem};
use eframe::egui::Ui;
use std::collections::{BTreeSet, HashMap};

type ComplaintCatRow<'a> = (
    String,
    &'a FormMasterItem,
    &'a FormMasterItem,
    &'a FormMasterItem,
    &'a FormMasterItem,
    &'a FormMasterItem,
    &'a FormMasterItem,
);

pub fn try_render(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) -> bool {
    let first = items[0];
    if first.section != "C" {
        return false;
    }

    match (first.principle_number, first.question_number) {
        (1, 1) => {
            render_p1_training_table(ui, exact_facts, salt);
            true
        }
        (2, 4) if first.indicator_type.eq_ignore_ascii_case("l") => {
            render_p2_reclaimed_table(ui, exact_facts, salt);
            true
        }
        (3, 1) if !first.indicator_type.eq_ignore_ascii_case("l") => {
            render_p3_wellbeing_tables(ui, items, exact_facts, tag_facts, strip_fn, salt);
            true
        }
        (3, 2) => {
            render_p3_retirement_table(ui, exact_facts, tag_facts, salt);
            true
        }
        (3, 5) => {
            render_p3_parental_table(ui, exact_facts, salt);
            true
        }
        (3, 13) | (5, 6) | (9, 3) => {
            render_cy_py_complaints_matrix(ui, items, exact_facts, salt);
            true
        }
        (5, 3) => {
            render_p5_remuneration_tables(ui, items, exact_facts, tag_facts, strip_fn, salt);
            true
        }
        _ => false,
    }
}

fn render_p1_training_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let segs = [
        ("Board of Directors", "D_BoardOfDirectorsSegment"),
        (
            "Key Managerial Personnel",
            "D_KeyManagerialPersonnelSegment",
        ),
        (
            "Employees other than BoD and KMPs",
            "D_EmployeesOtherThanBoDAndKMPsSegment",
        ),
        ("Workers", "D_WorkersSegment"),
    ];
    let rows: Vec<Vec<Option<CellData>>> = segs
        .iter()
        .map(|(label, ctx)| {
            vec![
                text_cell(label),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "TotalNumberOfTrainingAndAwarenessProgramsHeld",
                        ctx,
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "TopicsOrPrinciplesCoveredUnderTheTrainingAndItsImpact",
                        ctx,
                    ),
                    "string",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "PercentageOfPersonsInRespectiveCategoryCoveredByTheAwarenessProgrammes",
                        ctx,
                    ),
                    "number",
                ),
            ]
        })
        .collect();
    let avail = ui.available_width().max(460.0);
    let c0 = 175.0;
    let c1 = 100.0;
    let c3 = 100.0;
    let c2 = (avail - c0 - c1 - c3 - 20.0).max(180.0);
    render_matrix_table(
        ui,
        (salt, "p1_training"),
        &[
            "Segment",
            "Programmes Held",
            "Topics & Principles Covered",
            "% Covered",
        ],
        &[c0, c1, c2, c3],
        &rows,
    );
}

fn render_p2_reclaimed_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let cats = [
        (
            "Plastics (including packaging)",
            "D_PlasticsIncludingPackaging",
        ),
        ("E-waste", "D_EWaste"),
        ("Hazardous waste", "D_HazardousWaste"),
        ("Other waste", "D_OtherWaste1"),
    ];
    let rows: Vec<Vec<Option<CellData>>> = cats
        .iter()
        .map(|(label, ctx)| {
            let py_ctx = if ctx.ends_with('1') {
                "D_OtherWaste_PY1".to_string()
            } else {
                format!("{}_PY", ctx)
            };
            let cat_name = if *ctx == "D_OtherWaste1" {
                find_fact(exact_facts, "NameOfOtherWaste", ctx)
                    .map(|f| format!("Other: {}", f.raw_value))
                    .unwrap_or_else(|| (*label).to_string())
            } else {
                (*label).to_string()
            };
            vec![
                text_cell(&cat_name),
                fact_cell(find_fact(exact_facts, "AmountOfReUsed", ctx), "number"),
                fact_cell(find_fact(exact_facts, "AmountOfReUsed", &py_ctx), "number"),
                fact_cell(find_fact(exact_facts, "AmountOfRecycled", ctx), "number"),
                fact_cell(
                    find_fact(exact_facts, "AmountOfRecycled", &py_ctx),
                    "number",
                ),
                fact_cell(
                    find_fact(exact_facts, "AmountOfSafelyDisposed", ctx),
                    "number",
                ),
                fact_cell(
                    find_fact(exact_facts, "AmountOfSafelyDisposed", &py_ctx),
                    "number",
                ),
            ]
        })
        .collect();
    render_matrix_table(
        ui,
        (salt, "p2_reclaimed"),
        &[
            "Waste Category",
            "CY Re-Used",
            "PY Re-Used",
            "CY Recycled",
            "PY Recycled",
            "CY Disposed",
            "PY Disposed",
        ],
        &[180.0, 90.0, 90.0, 90.0, 90.0, 90.0, 90.0],
        &rows,
    );
}

fn render_p3_wellbeing_tables(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) {
    let headers = &[
        "Category",
        "Total (A)",
        "Health Ins. No. (B)",
        "Health %",
        "Accident Ins. No. (C)",
        "Accident %",
        "Maternity No. (D)",
        "Maternity %",
        "Paternity No. (E)",
        "Paternity %",
        "Day Care No. (F)",
        "Day Care %",
    ];
    let widths = &[
        195.0, 75.0, 82.0, 72.0, 82.0, 72.0, 82.0, 72.0, 82.0, 72.0, 82.0, 72.0,
    ];

    let build_rows = |seg: &str, suffix: &str| -> Vec<Vec<Option<CellData>>> {
        let genders = [
            ("Male", "D_Male"),
            ("Female", "D_Female"),
            ("Other", "D_OtherGender"),
            ("Total", "D_Gender"),
        ];
        genders
            .iter()
            .map(|(g_label, g_prefix)| {
                let total_ctx = if *g_prefix == "D_Gender" {
                    format!("D_Gender_Benefits_{}_{}", seg, suffix)
                } else {
                    format!("{}_TotalEmployeeBenefits_{}_{}", g_prefix, seg, suffix)
                };
                let ctx_for =
                    |benefit: &str| format!("{}_{}_{}_{}", g_prefix, benefit, seg, suffix);
                vec![
                    text_cell(&format!("{} — {}", seg, g_label)),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &total_ctx,
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("HealthInsurance"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "PercentageOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("HealthInsurance"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("AccidentInsurance"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "PercentageOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("AccidentInsurance"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("MaternityBenefits"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "PercentageOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("MaternityBenefits"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("PaternityBenefits"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "PercentageOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("PaternityBenefits"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "NumberOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("DayCareFacilities"),
                        ),
                        "number",
                    ),
                    fact_cell(
                        find_fact(
                            exact_facts,
                            "PercentageOfWellBeingOfEmployeesOrWorkers",
                            &ctx_for("DayCareFacilities"),
                        ),
                        "number",
                    ),
                ]
            })
            .collect()
    };

    render_table_subheading(
        ui,
        "a. Well-being of Employees — Permanent & Other than Permanent",
    );
    let mut emp_rows = build_rows("PermanentEmployees", "Table1A");
    emp_rows.extend(build_rows("OtherThanPermanentEmployees", "Table1A"));
    render_matrix_table(ui, (salt, "wb_emp"), headers, widths, &emp_rows);

    ui.add_space(6.0);
    render_table_subheading(
        ui,
        "b. Well-being of Workers — Permanent & Other than Permanent",
    );
    let mut wrk_rows = build_rows("PermanentWorkers", "Table1B");
    wrk_rows.extend(build_rows("OtherThanPermanentWorkers", "Table1B"));
    render_matrix_table(ui, (salt, "wb_wrk"), headers, widths, &wrk_rows);

    let c_items: Vec<&FormMasterItem> = items
        .iter()
        .copied()
        .filter(|it| it.followup_question_first.eq_ignore_ascii_case("c"))
        .collect();
    if !c_items.is_empty() {
        ui.add_space(6.0);
        render_table_subheading(ui, "c. Spending on measures towards well-being");
        render_cy_py_table(
            ui,
            &c_items,
            exact_facts,
            tag_facts,
            None,
            strip_fn,
            (salt.0, salt.1, salt.2, 1001),
        );
    }
}

fn render_p3_retirement_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let fixed = [
        ("PF", "D_ProvidentFund"),
        ("Gratuity", "D_Gratuity"),
        ("ESI", "D_ESI"),
    ];
    let mut rows: Vec<Vec<Option<CellData>>> = fixed
        .iter()
        .map(|(label, ctx)| {
            let py = format!("{}_PY", ctx);
            vec![
                text_cell(label),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "NumberOfEmployeesCoveredAsPercentageOfTotalEmployees",
                        ctx,
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "NumberOfEmployeesCoveredAsPercentageOfTotalWorker",
                        ctx,
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(exact_facts, "DeductedAndDepositedWithTheAuthority", ctx),
                    "string",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "NumberOfEmployeesCoveredAsPercentageOfTotalEmployees",
                        &py,
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        "NumberOfEmployeesCoveredAsPercentageOfTotalWorker",
                        &py,
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(exact_facts, "DeductedAndDepositedWithTheAuthority", &py),
                    "string",
                ),
            ]
        })
        .collect();

    let mut other_indices = BTreeSet::new();
    if let Some(list) = tag_facts.get("NameOfOtherRetirementBenefits") {
        for f in list {
            if let Some(idx) = f.context_ref.strip_prefix("D_OtherRetirementBenefits")
                && !idx.starts_with("_PY")
                && idx.chars().all(|c| c.is_ascii_digit())
            {
                other_indices.insert(idx.to_string());
            }
        }
    }
    for idx in other_indices {
        let ctx = format!("D_OtherRetirementBenefits{}", idx);
        let py = format!("D_OtherRetirementBenefits_PY{}", idx);
        let name = find_fact(exact_facts, "NameOfOtherRetirementBenefits", &ctx)
            .map(|f| f.raw_value.clone())
            .unwrap_or_else(|| format!("Other Benefit {}", idx));
        rows.push(vec![
            text_cell(&name),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfEmployeesCoveredAsPercentageOfTotalEmployees",
                    &ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfEmployeesCoveredAsPercentageOfTotalWorker",
                    &ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "DeductedAndDepositedWithTheAuthority", &ctx),
                "string",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfEmployeesCoveredAsPercentageOfTotalEmployees",
                    &py,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "NumberOfEmployeesCoveredAsPercentageOfTotalWorker",
                    &py,
                ),
                "number",
            ),
            fact_cell(
                find_fact(exact_facts, "DeductedAndDepositedWithTheAuthority", &py),
                "string",
            ),
        ]);
    }

    render_matrix_table(
        ui,
        (salt, "retirement"),
        &[
            "Benefits",
            "CY Employees %",
            "CY Workers %",
            "CY Deposited?",
            "PY Employees %",
            "PY Workers %",
            "PY Deposited?",
        ],
        &[150.0, 95.0, 95.0, 95.0, 95.0, 95.0, 95.0],
        &rows,
    );
}

fn render_p3_parental_table(
    ui: &mut Ui,
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let mut rows = Vec::new();
    for (g_label, ctx) in [
        ("Male", "D_Male"),
        ("Female", "D_Female"),
        ("Other", "D_OtherGender"),
        ("Total", "D_Gender"),
    ] {
        rows.push(vec![
            text_cell(g_label),
            fact_cell(
                find_fact(
                    exact_facts,
                    "ReturnToWorkRatePermanentEmployeesThatTookParentalLeave",
                    ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "RetentionRatesPermanentEmployeesThatTookParentalLeave",
                    ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "ReturnToWorkRatePermanentWorkersThatTookParentalLeave",
                    ctx,
                ),
                "number",
            ),
            fact_cell(
                find_fact(
                    exact_facts,
                    "RetentionRatesPermanentWorkersThatTookParentalLeave",
                    ctx,
                ),
                "number",
            ),
        ]);
    }
    let avail = ui.available_width().max(480.0);
    let c0 = 100.0;
    let cw = ((avail - c0 - 20.0) / 4.0).max(95.0);
    render_matrix_table(
        ui,
        (salt, "parental"),
        &[
            "Gender",
            "Perm. Emp. Return %",
            "Perm. Emp. Retention %",
            "Perm. Wrk. Return %",
            "Perm. Wrk. Retention %",
        ],
        &[c0, cw, cw, cw, cw],
        &rows,
    );
}

fn flush_complaints_cat<'a>(
    cat: &str,
    buf: &[&'a FormMasterItem],
    out: &mut Vec<ComplaintCatRow<'a>>,
) {
    if buf.len() >= 6 && !cat.is_empty() {
        let mut cy_filed = None;
        let mut py_filed = None;
        let mut cy_pend = None;
        let mut py_pend = None;
        let mut cy_rem = None;
        let mut py_rem = None;
        for &it in buf {
            let is_py = it.append_to_field.as_deref() == Some("_PY")
                || it.standard_context_reference.ends_with("_PY");
            let tag = it.standard_indicator_capmkt.as_str();
            if tag.contains("Filed") || tag.contains("Received") {
                if is_py {
                    py_filed = Some(it);
                } else {
                    cy_filed = Some(it);
                }
            } else if tag.contains("Pending") {
                if is_py {
                    py_pend = Some(it);
                } else {
                    cy_pend = Some(it);
                }
            } else if tag.contains("Remark") {
                if is_py {
                    py_rem = Some(it);
                } else {
                    cy_rem = Some(it);
                }
            }
        }
        if let (Some(cf), Some(cp), Some(cr), Some(pf), Some(pp), Some(pr)) =
            (cy_filed, cy_pend, cy_rem, py_filed, py_pend, py_rem)
        {
            out.push((cat.to_string(), cf, cp, cr, pf, pp, pr));
        }
    }
}

fn render_cy_py_complaints_matrix(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    salt: (&str, i16, i16, i16),
) {
    let mut categories: Vec<ComplaintCatRow<'_>> = Vec::new();
    let mut current_cat = String::new();
    let mut cat_items: Vec<&FormMasterItem> = Vec::new();

    for &item in items {
        let has_tag = !item.standard_indicator_capmkt.trim().is_empty()
            && item.standard_indicator_capmkt.trim() != "0";
        if !has_tag {
            let txt = item.question_text.trim();
            if item.question_order > 1
                && !txt.eq_ignore_ascii_case("filed during the year")
                && !txt.eq_ignore_ascii_case("received during the year")
                && !txt.eq_ignore_ascii_case("pending resolution at the end of year")
                && !txt.eq_ignore_ascii_case("pending resolution at end of year")
                && !txt.eq_ignore_ascii_case("remarks")
                && !txt.eq_ignore_ascii_case("remark")
            {
                flush_complaints_cat(&current_cat, &cat_items, &mut categories);
                cat_items.clear();
                current_cat = txt.to_string();
            }
        } else {
            cat_items.push(item);
        }
    }
    flush_complaints_cat(&current_cat, &cat_items, &mut categories);

    let rows: Vec<Vec<Option<CellData>>> = categories
        .into_iter()
        .map(|(cat, cf, cp, cr, pf, pp, pr)| {
            vec![
                text_cell(&cat),
                fact_cell(
                    find_fact(
                        exact_facts,
                        cf.standard_indicator_capmkt.trim(),
                        cf.standard_context_reference.trim(),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        cp.standard_indicator_capmkt.trim(),
                        cp.standard_context_reference.trim(),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        cr.standard_indicator_capmkt.trim(),
                        cr.standard_context_reference.trim(),
                    ),
                    "string",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        pf.standard_indicator_capmkt.trim(),
                        pf.standard_context_reference.trim(),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        pp.standard_indicator_capmkt.trim(),
                        pp.standard_context_reference.trim(),
                    ),
                    "number",
                ),
                fact_cell(
                    find_fact(
                        exact_facts,
                        pr.standard_indicator_capmkt.trim(),
                        pr.standard_context_reference.trim(),
                    ),
                    "string",
                ),
            ]
        })
        .collect();

    render_matrix_table(
        ui,
        (salt, "complaints_cy_py"),
        &[
            "Category",
            "CY Filed",
            "CY Pending",
            "CY Remarks",
            "PY Filed",
            "PY Pending",
            "PY Remarks",
        ],
        &[175.0, 80.0, 85.0, 145.0, 80.0, 85.0, 145.0],
        &rows,
    );
}

fn render_p5_remuneration_tables(
    ui: &mut Ui,
    items: &[&FormMasterItem],
    exact_facts: &HashMap<(&str, &str), Vec<&FormFactRow>>,
    tag_facts: &HashMap<&str, Vec<&FormFactRow>>,
    strip_fn: fn(&str) -> &str,
    salt: (&str, i16, i16, i16),
) {
    render_table_subheading(ui, "a. Median remuneration / salary / wages");
    let segs = [
        (
            "Board of Directors (BoD)",
            "NumberOfBoardOfDirectorsForRemunerationOrSalaryOrWages",
            "MedianOfRemunerationOrSalaryOrWagesOfBoardOfDirectors",
        ),
        (
            "Key Managerial Personnel",
            "NumberOfKeyManagerialPersonnelForRemunerationOrSalaryOrWages",
            "MedianOfRemunerationOrSalaryOrWagesOfKeyManagerialPersonnel",
        ),
        (
            "Employees other than BoD and KMP",
            "NumberOfEmployeesOtherThanBodAndKMPForRemunerationOrSalaryOrWages",
            "MedianOfRemunerationOrSalaryOrWagesOfEmployeesOtherThanBodAndKMP",
        ),
        (
            "Workers",
            "NumberOfWorkersForRemunerationOrSalaryOrWages",
            "MedianOfRemunerationOrSalaryOrWagesOfWorkers",
        ),
    ];
    let rows: Vec<Vec<Option<CellData>>> = segs
        .iter()
        .map(|(label, num_tag, med_tag)| {
            vec![
                text_cell(label),
                fact_cell(find_fact(exact_facts, num_tag, "D_Male_p5"), "number"),
                fact_cell(find_fact(exact_facts, med_tag, "D_Male_p5"), "number"),
                fact_cell(find_fact(exact_facts, num_tag, "D_Female_p5"), "number"),
                fact_cell(find_fact(exact_facts, med_tag, "D_Female_p5"), "number"),
                fact_cell(
                    find_fact(exact_facts, num_tag, "D_OtherGender_p5"),
                    "number",
                ),
                fact_cell(
                    find_fact(exact_facts, med_tag, "D_OtherGender_p5"),
                    "number",
                ),
            ]
        })
        .collect();
    render_matrix_table(
        ui,
        (salt, "p5_median_rem"),
        &[
            "Category",
            "Male No.",
            "Male Median",
            "Female No.",
            "Female Median",
            "Other No.",
            "Other Median",
        ],
        &[185.0, 80.0, 105.0, 80.0, 105.0, 80.0, 105.0],
        &rows,
    );

    let b_items: Vec<&FormMasterItem> = items
        .iter()
        .copied()
        .filter(|it| it.followup_question_first.eq_ignore_ascii_case("b"))
        .collect();
    if !b_items.is_empty() {
        ui.add_space(6.0);
        render_table_subheading(ui, "b. Gross wages paid to females as % of total wages");
        render_cy_py_table(
            ui,
            &b_items,
            exact_facts,
            tag_facts,
            None,
            strip_fn,
            (salt.0, salt.1, salt.2, 1002),
        );
    }
}
