for file in \
    00_teardown_and_extensions.sql \
    01_file_list.sql \
    02_form_master.sql \
    03_entity_list.sql \
    04_brsr_facts.sql \
    05_unit_conversions.sql \
    06_industry_master.sql \
    07_cluster_tuning.sql; do
    echo "▶ Executing $file..."
    podman exec -i postgres_contain psql -U rockhopper_dev -d brsr_db_test < "$file"
done
