#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CONTAINER="postgres_contain"
DB_USER="rockhopper_dev"
DB_NAME="brsr_db_test"

XBRL_TAG_RULES_CSV="$SCRIPT_DIR/seeds/xbrl_tag_rules.csv"
FORM_LAYOUT_CSV="$SCRIPT_DIR/seeds/form_layout.csv"
INDUSTRY_MASTER_CSV="$SCRIPT_DIR/seeds/industry_master.csv"
PIN_CODE_CSV="$SCRIPT_DIR/seeds/pin_code.csv"
UNIT_CONVERSION_CSV="$SCRIPT_DIR/seeds/unit_conversions.csv"

SQL_FILES=(
    "00_teardown_and_extensions.sql"
    "01_file_list.sql"
    "02a_xbrl_tag_rules.sql"
    "02b_form_layout.sql"
    "03_industry_master.sql"
    "04_exchange_industry_master.sql"
    "05_geo_pincode.sql"
    "06_unit_conversions.sql"
    "07_entity_list.sql"
    "08_brsr_facts.sql"
    "09_analytics_views.sql"
    "10_cluster_tuning.sql"
    "99_data_dictionary.sql"
)

run_psql() {
    podman exec -i "$CONTAINER" psql -v ON_ERROR_STOP=1 -U "$DB_USER" -d "$DB_NAME" "$@"
}

for file in "${SQL_FILES[@]}"; do
    echo "▶ Executing $file..."
    run_psql < "$SCRIPT_DIR/$file"

    # Seed xbrl_tag_rules immediately after its table creation
    if [ "$file" = "02a_xbrl_tag_rules.sql" ]; then
        if [ -f "$XBRL_TAG_RULES_CSV" ]; then
            echo "📥 Importing $XBRL_TAG_RULES_CSV into xbrl_tag_rules..."
            run_psql -c \
                "\copy xbrl_tag_rules (raw_tag, raw_context, std_tag, std_context, is_axis_increment, section, principle_number, sub_section, indicator_type, friendly_grouping, is_legacy) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '');" < "$XBRL_TAG_RULES_CSV"
            echo "✓ xbrl_tag_rules imported."
        else
            echo "⚠ Warning: $XBRL_TAG_RULES_CSV not found, skipping."
        fi
    fi

    # Seed form_layout immediately after its table creation
    if [ "$file" = "02b_form_layout.sql" ]; then
        if [ -f "$FORM_LAYOUT_CSV" ]; then
            echo "📥 Importing $FORM_LAYOUT_CSV into form_layout..."
            run_psql -c \
                "\copy form_layout (question_key, sort_order, section, principle_number, sub_section, indicator_type, question_in_form, question_number, followup_group, render_kind, section_text, principle_text, sub_section_text, indicator_text, question_text, std_tag, std_context, response_datatype, is_axis_increment, append_to_field, friendly_grouping, visibility_parent_key, visibility_trigger, input_text) FROM STDIN WITH (FORMAT csv, HEADER true, NULL 'NULL', FORCE_NULL (question_in_form, followup_group, section_text, principle_text, sub_section_text, indicator_text, std_tag, std_context, append_to_field, friendly_grouping, visibility_parent_key, visibility_trigger, input_text));" < "$FORM_LAYOUT_CSV"
            echo "✓ form_layout imported."
        else
            echo "⚠ Warning: $FORM_LAYOUT_CSV not found, skipping."
        fi
    fi

    # Seed industry_master immediately after its table creation
    if [ "$file" = "03_industry_master.sql" ]; then
        if [ -f "$INDUSTRY_MASTER_CSV" ]; then
            echo "📥 Importing $INDUSTRY_MASTER_CSV into industry_master..."
            run_psql -c \
                "\copy industry_master (nic_code, division_code, division_name, section_code, section_name, esg_sector) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '');" < "$INDUSTRY_MASTER_CSV"
            echo "✓ industry_master imported."
        else
            echo "⚠ Warning: $INDUSTRY_MASTER_CSV not found, skipping."
        fi
    fi

    # Seed geo_pincode immediately after its table creation
    if [ "$file" = "05_geo_pincode.sql" ]; then
        if [ -f "$PIN_CODE_CSV" ]; then
            echo "📥 Importing $PIN_CODE_CSV into geo_pincode..."
            run_psql -c \
                "\copy geo_pincode (country_code, postal_code, place_name, admin_name1, admin_code1, admin_name2, admin_code2, admin_name3, admin_code3, latitude, longitude, accuracy) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '', FORCE_NULL (admin_name1, admin_code1, admin_name2, admin_code2, admin_name3, admin_code3, accuracy));" < "$PIN_CODE_CSV"
            echo "✓ geo_pincode imported."
        else
            echo "⚠ Warning: $PIN_CODE_CSV not found, skipping."
        fi
    fi

    # Seed unit_conversions immediately after its table creation
    if [ "$file" = "06_unit_conversions.sql" ]; then
        if [ -f "$UNIT_CONVERSION_CSV" ]; then
            echo "📥 Importing $UNIT_CONVERSION_CSV into unit_conversions..."
            run_psql -c \
                "\copy unit_conversions (metric_category, raw_unit, canonical_unit, conversion_factor, notes) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '', FORCE_NULL (notes));" < "$UNIT_CONVERSION_CSV"
            echo "✓ unit_conversions imported."
        else
            echo "⚠ Warning: $UNIT_CONVERSION_CSV not found, skipping."
        fi
    fi
done

echo "🎉 Database reset, seeding, views, and data dictionary complete!"