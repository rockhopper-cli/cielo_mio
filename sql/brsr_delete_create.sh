#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CONTAINER="postgres_contain"
DB_USER="rockhopper_dev"
DB_NAME="brsr_db_test"

FORM_MASTER_CSV="$SCRIPT_DIR/seeds/form_master.csv"
INDUSTRY_MASTER_CSV="$SCRIPT_DIR/seeds/industry_master.csv"
PIN_CODE_CSV="$SCRIPT_DIR/seeds/pin_code.csv"
UNIT_CONVERSION_CSV="$SCRIPT_DIR/seeds/unit_conversions.csv"

SQL_FILES=(
    "00_teardown_and_extensions.sql"
    "01_file_list.sql"
    "02_form_master.sql"
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

for file in "${SQL_FILES[@]}"; do
    echo "▶ Executing $file..."
    podman exec -i "$CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" < "$SCRIPT_DIR/$file"

    # Seed form_master immediately after its table creation
    if [ "$file" = "02_form_master.sql" ]; then
        if [ -f "$FORM_MASTER_CSV" ]; then
            echo "📥 Importing $FORM_MASTER_CSV into form_master..."
            podman exec -i "$CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c \
                "\copy form_master (question_key, section, principle_number, sub_section, indicator_type, question_in_form, question_number, followup_question_first, followup_question_second, question_order, question_type, response_datatype, response_required, calculation, contextref_increment, visibility_dependence, visibility_trigger, total_field, friendly_grouping, field_flag, anchor_reference, append_to_field, legacy, section_text, principle_text, sub_section_text, indicator_text, question_text, indicator_capmkt, context_reference, standard_indicator_capmkt, standard_context_reference, response_options, input_text, formula, comments) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '');" < "$FORM_MASTER_CSV"
            echo "✓ form_master imported."
        else
            echo "⚠ Warning: $FORM_MASTER_CSV not found, skipping."
        fi
    fi

    # Seed industry_master immediately after its table creation
    if [ "$file" = "03_industry_master.sql" ]; then
        if [ -f "$INDUSTRY_MASTER_CSV" ]; then
            echo "📥 Importing $INDUSTRY_MASTER_CSV into industry_master..."
            podman exec -i "$CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c \
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
            podman exec -i "$CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c \
                "\copy geo_pincode (country_code, postal_code, place_name, admin_name1, admin_code1, admin_name2, admin_code2, admin_name3, admin_code3, latitude, longitude, accuracy) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '');" < "$PIN_CODE_CSV"
            echo "✓ geo_pincode imported."
        else
            echo "⚠ Warning: $PIN_CODE_CSV not found, skipping."
        fi
    fi

    # Seed unit_conversions immediately after its table creation
    if [ "$file" = "06_unit_conversions.sql" ]; then
        if [ -f "$UNIT_CONVERSION_CSV" ]; then
            echo "📥 Importing $UNIT_CONVERSION_CSV into unit_conversions..."
            podman exec -i "$CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c \
                "\copy unit_conversions (metric_category, raw_unit, canonical_unit, conversion_factor, notes) FROM STDIN WITH (FORMAT csv, HEADER true, NULL '');" < "$UNIT_CONVERSION_CSV"
            echo "✓ unit_conversions imported."
        else
            echo "⚠ Warning: $UNIT_CONVERSION_CSV not found, skipping."
        fi
    fi
done

echo "🎉 Database reset, seeding, views, and data dictionary complete!"