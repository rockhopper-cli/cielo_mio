-- ============================================================================
-- 00_teardown_and_extensions.sql
-- Drop existing tables in reverse dependency order and initialize extensions
-- ============================================================================

DROP TABLE IF EXISTS brsr_facts CASCADE;
DROP TABLE IF EXISTS entity_list CASCADE;
DROP TABLE IF EXISTS file_list CASCADE;
DROP TABLE IF EXISTS form_master CASCADE;
DROP TABLE IF EXISTS unit_conversions CASCADE;
DROP TABLE IF EXISTS industry_master CASCADE;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "btree_gist";  -- Temporal GIST exclusion constraints
CREATE EXTENSION IF NOT EXISTS "pg_trgm";     -- Fast ILIKE '%...%' substring scans
CREATE EXTENSION IF NOT EXISTS "vector";      -- RAG embeddings