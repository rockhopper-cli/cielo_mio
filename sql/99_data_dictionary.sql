-- create table - Static Snapshot of Database Structure & Column Comments
CREATE TABLE data_dictionary AS
SELECT 
    cols.table_name,
    cols.ordinal_position AS col_order,
    cols.column_name, 
    cols.data_type, 
    cols.character_maximum_length AS max_length,
    cols.numeric_precision,
    cols.numeric_scale,
    cols.is_nullable,
    cols.column_default,
    pg_catalog.col_description(c.oid, a.attnum) AS column_comment
FROM information_schema.columns cols
JOIN pg_catalog.pg_class c 
    ON c.relname = cols.table_name
JOIN pg_catalog.pg_namespace n 
    ON n.oid = c.relnamespace AND n.nspname = cols.table_schema
JOIN pg_catalog.pg_attribute a 
    ON a.attrelid = c.oid AND a.attname = cols.column_name
WHERE cols.table_schema = 'public'
  AND c.relkind IN ('r', 'p')
  AND a.attnum > 0
  AND NOT a.attisdropped
ORDER BY cols.table_name, cols.ordinal_position;

-- index
CREATE INDEX idx_data_dict_table ON data_dictionary(table_name);