-- create table - NIC Macro Divisions & 5-Digit Specific Seeds
CREATE TABLE industry_master (
    industry_id UUID PRIMARY KEY DEFAULT uuidv7(),
    nic_code VARCHAR(10) NOT NULL UNIQUE,
    division_code VARCHAR(5) NOT NULL,
    division_name VARCHAR(255) NOT NULL,
    section_code VARCHAR(2) NOT NULL,
    section_name VARCHAR(255) NOT NULL,
    esg_sector VARCHAR(100) NOT NULL
);

-- index
CREATE INDEX idx_industry_division ON industry_master(division_code);

-- comments 8
COMMENT ON TABLE industry_master IS 'Master taxonomy mapping Indian NIC codes (from CIN chars 2-6) to MOSPI divisions and custom ESG benchmarking peer groups.';
COMMENT ON COLUMN industry_master.industry_id IS 'Unique UUID for the industry mapping record';
COMMENT ON COLUMN industry_master.nic_code IS 'Unique Key. 5-digit NIC code (from CIN chars 2-6, e.g. 62011) or 2-digit fallback code (e.g. 62).';
COMMENT ON COLUMN industry_master.division_code IS '2-digit MOSPI Division number (01 to 99), representing the broad economic activity.';
COMMENT ON COLUMN industry_master.division_name IS 'Official MOSPI title describing the 2-digit division.';
COMMENT ON COLUMN industry_master.section_code IS 'Top-level single-letter category (A to U) in the NIC/ISIC classification hierarchy.';
COMMENT ON COLUMN industry_master.section_name IS 'Official macro-level title corresponding to the section letter (e.g., Manufacturing, Information and Communication).';
COMMENT ON COLUMN industry_master.esg_sector IS 'Actionable ESG peer-group label used for BRSR analytics, emissions intensity comparisons, and AI prompts.';

