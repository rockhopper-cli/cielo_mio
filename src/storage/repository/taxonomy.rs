// src/storage/repository/taxonomy.rs

use crate::ui::state::FormMasterItem;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};

/// Fetches physical unit conversion multipliers from unit_conversions
pub async fn fetch_unit_conversions(
    pool: &PgPool,
) -> Result<Vec<(String, String, Decimal)>, sqlx::Error> {
    let rows =
        sqlx::query("SELECT raw_unit, canonical_unit, conversion_factor FROM unit_conversions")
            .fetch_all(pool)
            .await?;

    let mut result = Vec::with_capacity(rows.len());
    for r in rows {
        result.push((
            r.get("raw_unit"),
            r.get("canonical_unit"),
            r.get("conversion_factor"),
        ));
    }
    Ok(result)
}

/// Fetches NIC and division mapping rules from industry_master
pub async fn fetch_industry_master(
    pool: &PgPool,
) -> Result<Vec<(String, String, String, String)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT nic_code, division_code, division_name, esg_sector FROM industry_master",
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for r in rows {
        result.push((
            r.get("nic_code"),
            r.get("division_code"),
            r.get("division_name"),
            r.get("esg_sector"),
        ));
    }
    Ok(result)
}

/// Fetches statutory form taxonomy and template structure from form_master
pub async fn fetch_form_template(pool: &PgPool) -> Result<Vec<FormMasterItem>, sqlx::Error> {
    let sql = r#"
        SELECT 
            question_key, section, principle_number, sub_section, indicator_type,
            question_in_form, question_number,
            COALESCE(followup_group, '0') AS followup_question_first,
            '0'::varchar AS followup_question_second,
            ROW_NUMBER() OVER (
                PARTITION BY section, principle_number, sub_section, question_number
                ORDER BY sort_order
            )::smallint AS question_order,
            CASE render_kind
                WHEN 'heading' THEN 'question'
                WHEN 'scalar'  THEN 'question'
                ELSE 'detail'
            END AS question_type,
            response_datatype,
            is_axis_increment AS contextref_increment,
            FALSE AS total_field,
            friendly_grouping,
            CASE render_kind
                WHEN 'matrix_9p'  THEN 2::smallint
                WHEN 'axis_col'   THEN 1::smallint
                WHEN 'table_cell' THEN 1::smallint
                ELSE 0::smallint
            END AS field_flag,
            0::smallint AS anchor_reference,
            append_to_field,
            COALESCE(NULLIF(section_text, ''), 'SECTION ' || section) AS section_text,
            COALESCE(NULLIF(principle_text, ''), 'General') AS principle_text,
            COALESCE(NULLIF(sub_section_text, ''), 'General') AS sub_section_text,
            COALESCE(NULLIF(indicator_text, ''), 'General') AS indicator_text,
            question_text,
            COALESCE(std_tag, '') AS standard_indicator_capmkt,
            COALESCE(std_context, '') AS standard_context_reference,
            input_text,
            COALESCE(visibility_parent_key, '0') AS visibility_dependence,
            visibility_trigger
        FROM form_layout
        ORDER BY sort_order ASC;
    "#;

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    let mut template = Vec::with_capacity(rows.len());

    for r in rows {
        let field_flag = r
            .try_get::<i16, _>("field_flag")
            .or_else(|_| r.try_get::<i32, _>("field_flag").map(|v| v as i16))
            .unwrap_or(0);

        let anchor_reference = r
            .try_get::<i16, _>("anchor_reference")
            .or_else(|_| r.try_get::<i32, _>("anchor_reference").map(|v| v as i16))
            .unwrap_or(0);

        let p_num = r
            .try_get::<i16, _>("principle_number")
            .or_else(|_| r.try_get::<i32, _>("principle_number").map(|v| v as i16))
            .unwrap_or(0);

        let sub_sec = r
            .try_get::<i16, _>("sub_section")
            .or_else(|_| r.try_get::<i32, _>("sub_section").map(|v| v as i16))
            .unwrap_or(0);

        let q_in_form = r.try_get::<i16, _>("question_in_form").ok().or_else(|| {
            r.try_get::<i32, _>("question_in_form")
                .ok()
                .map(|v| v as i16)
        });

        let q_num = r
            .try_get::<i16, _>("question_number")
            .or_else(|_| r.try_get::<i32, _>("question_number").map(|v| v as i16))
            .unwrap_or(0);

        let q_order = r
            .try_get::<i16, _>("question_order")
            .or_else(|_| r.try_get::<i32, _>("question_order").map(|v| v as i16))
            .unwrap_or(0);

        template.push(FormMasterItem {
            question_key: r.get("question_key"),
            section: r.get("section"),
            principle_number: p_num,
            sub_section: sub_sec,
            indicator_type: r.get("indicator_type"),
            question_in_form: q_in_form,
            question_number: q_num,
            followup_question_first: r.get("followup_question_first"),
            followup_question_second: r.get("followup_question_second"),
            question_order: q_order,
            question_type: r.get("question_type"),
            response_datatype: r.get("response_datatype"),
            contextref_increment: r.get("contextref_increment"),
            total_field: r.get("total_field"),
            friendly_grouping: r.get("friendly_grouping"),
            field_flag,
            anchor_reference,
            append_to_field: r.get("append_to_field"),
            section_text: clean_text(&r.get::<String, _>("section_text")),
            principle_text: clean_text(&r.get::<String, _>("principle_text")),
            sub_section_text: clean_text(&r.get::<String, _>("sub_section_text")),
            indicator_text: clean_text(&r.get::<String, _>("indicator_text")),
            question_text: clean_text(&r.get::<String, _>("question_text")),
            standard_indicator_capmkt: r.get("standard_indicator_capmkt"),
            standard_context_reference: r.get("standard_context_reference"),
            input_text: r
                .get::<Option<String>, _>("input_text")
                .map(|s| clean_text(&s)),
            visibility_dependence: r.get("visibility_dependence"),
            visibility_trigger: r.get("visibility_trigger"),
        });
    }

    Ok(template)
}

fn clean_text(s: &str) -> String {
    s.replace('Â', "")
        .replace('\u{00A0}', " ")
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}
