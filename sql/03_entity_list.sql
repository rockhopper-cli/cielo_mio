-- ============================================================================
-- 03_entity_list.sql
-- Company Master Disclosures & Temporal Boundaries
-- ============================================================================

CREATE TABLE entity_list (
    entity_id UUID PRIMARY KEY DEFAULT uuidv7(),
    section VARCHAR(10) NOT NULL DEFAULT 'A',
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 1,
    friendly_group VARCHAR(255) NOT NULL DEFAULT 'entity_list',
    stock_market VARCHAR(50) NOT NULL,
    file_uuid UUID NOT NULL REFERENCES file_list(file_id) ON DELETE CASCADE,
    corporate_id_number VARCHAR(100) NOT NULL,        -- Fix C: Accommodates composite/extended CIN strings
    company_name VARCHAR(255) NOT NULL,
    address_corporate_office TEXT NULL,
    address_registered_office TEXT NULL,
    contact_person_name VARCHAR(500) NULL,
    contact_person_phone VARCHAR(255) NULL,
    contact_person_email VARCHAR(255) NULL,
    company_email VARCHAR(255) NULL,
    company_telephone VARCHAR(255) NULL,
    website VARCHAR(255) NULL,
    reporting_boundary VARCHAR(50) NULL DEFAULT 'Standalone',
    financial_year_start DATE NULL,
    financial_year_end DATE NULL,
    incorporation_date DATE NULL,
    paid_up_shares_value NUMERIC(20, 4) NULL,
    nic_code VARCHAR(10) NULL,
    industry_division VARCHAR(120) NULL,
    industry_sector VARCHAR(80) NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    -- Fix C: Expressions correctly enclosed in parentheses for valid PostgreSQL GIST syntax
    CONSTRAINT no_overlapping_filings EXCLUDE USING gist (
        corporate_id_number WITH =,
        (COALESCE(reporting_boundary, 'Standalone')) WITH =,
        (daterange(financial_year_start, financial_year_end, '[]')) WITH &&
    ) WHERE (financial_year_start IS NOT NULL AND financial_year_end IS NOT NULL)
);

-- Indexes

CREATE INDEX idx_entity_cin
ON entity_list (corporate_id_number);

CREATE INDEX idx_entity_file_uuid
ON entity_list (file_uuid);

CREATE INDEX idx_entity_sector
ON entity_list (industry_sector);

-- ----------------------------------------------------------------------------
-- 3. ENTITY_LIST (27 Comments)
-- ----------------------------------------------------------------------------
COMMENT ON TABLE entity_list IS 'Horizontal master record of corporate profiles, financial periods, and NIC industry classifications';
COMMENT ON COLUMN entity_list.entity_id IS 'Unique UUID identifier for the company filing record';
COMMENT ON COLUMN entity_list.section IS 'BRSR Section identifier (Default: A)';
COMMENT ON COLUMN entity_list.principle_number IS 'Principle number (Default: 0 for Section A)';
COMMENT ON COLUMN entity_list.sub_section IS 'Sub-section identifier (Default: 1 for General Disclosures)';
COMMENT ON COLUMN entity_list.friendly_group IS 'Grouping identifier (Default: entity_list)';
COMMENT ON COLUMN entity_list.stock_market IS 'Exchange source where filing was retrieved (BSE or NSE)';
COMMENT ON COLUMN entity_list.file_uuid IS 'Foreign key referencing file_list(file_id) to trace the source filing file';
COMMENT ON COLUMN entity_list.corporate_id_number IS 'Corporate Identity Number (CIN) issued by MCA or filing identifier';
COMMENT ON COLUMN entity_list.company_name IS 'Registered name of the company';
COMMENT ON COLUMN entity_list.address_corporate_office IS 'Full corporate office address';
COMMENT ON COLUMN entity_list.address_registered_office IS 'Full registered office address';
COMMENT ON COLUMN entity_list.contact_person_name IS 'Name of the designated BRSR contact person';
COMMENT ON COLUMN entity_list.contact_person_phone IS 'Contact person telephone/mobile number';
COMMENT ON COLUMN entity_list.contact_person_email IS 'Direct email ID of the contact person';
COMMENT ON COLUMN entity_list.company_email IS 'Official general corporate email ID';
COMMENT ON COLUMN entity_list.company_telephone IS 'Official company landline/switchboard number';
COMMENT ON COLUMN entity_list.website IS 'Official corporate website URL';
COMMENT ON COLUMN entity_list.reporting_boundary IS 'Scope of disclosures: Standalone or Consolidated';
COMMENT ON COLUMN entity_list.financial_year_start IS 'Start date of the reporting financial year';
COMMENT ON COLUMN entity_list.financial_year_end IS 'End date of the reporting financial year';
COMMENT ON COLUMN entity_list.incorporation_date IS 'Date of company incorporation';
COMMENT ON COLUMN entity_list.paid_up_shares_value IS 'Paid-up share capital = Total Issued Shares * Face Value (Par Value)';
COMMENT ON COLUMN entity_list.nic_code IS '5-digit or 2-digit National Industrial Classification code derived from CIN characters 2-6';
COMMENT ON COLUMN entity_list.industry_division IS 'MOSPI 2-digit division title representing broad economic activity';
COMMENT ON COLUMN entity_list.industry_sector IS 'Peer group category used for cross-company ESG analytics and intensity benchmarking';
COMMENT ON COLUMN entity_list.created_at IS 'Timestamp when the entity master record was inserted';
