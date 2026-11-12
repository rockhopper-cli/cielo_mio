-- start-from-scratch

DROP VIEW IF EXISTS view_sector_benchmarks CASCADE;
DROP VIEW IF EXISTS view_company_esg_kpis CASCADE;

DROP TABLE IF EXISTS brsr_facts CASCADE;
DROP TABLE IF EXISTS entity_list CASCADE;
DROP TABLE IF EXISTS file_list CASCADE;
DROP TABLE IF EXISTS form_master CASCADE;
DROP TABLE IF EXISTS unit_conversions CASCADE;
DROP TABLE IF EXISTS exchange_industry_master CASCADE;
DROP TABLE IF EXISTS geo_pincode CASCADE;
DROP TABLE IF EXISTS industry_master CASCADE;
DROP TABLE IF EXISTS data_dictionary CASCADE;

DROP FUNCTION IF EXISTS trg_resolve_entity_coordinates() CASCADE;
DROP FUNCTION IF EXISTS trg_enrich_entity_industry() CASCADE;
DROP FUNCTION IF EXISTS sync_entity_industries() CASCADE;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "btree_gist";    -- Temporal GIST exclusion constraints
CREATE EXTENSION IF NOT EXISTS "pg_trgm";       -- Fast ILIKE '%...%' substring scans
CREATE EXTENSION IF NOT EXISTS "vector";        -- RAG embeddings
