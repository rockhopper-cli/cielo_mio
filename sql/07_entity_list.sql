-- create table - Company Master Disclosures & Temporal Boundaries
CREATE TABLE entity_list (
    entity_id UUID PRIMARY KEY DEFAULT uuidv7(),
    section VARCHAR(10) NOT NULL DEFAULT 'A',
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 1,
    friendly_group VARCHAR(255) NOT NULL DEFAULT 'entity_list',
    stock_market VARCHAR(50) NOT NULL,
    file_uuid UUID NOT NULL REFERENCES file_list(file_id) ON DELETE CASCADE,
    corporate_id_number VARCHAR(100) NOT NULL,
    isin VARCHAR(12) NULL,
    nse_symbol VARCHAR(50) NULL,
    bse_scrip_code VARCHAR(20) NULL,
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
    nic_code VARCHAR(5) GENERATED ALWAYS AS (
        substring(UPPER(TRIM(corporate_id_number)) FROM '^[LU]([0-9]{5})[A-Z]{2}[0-9]{4}[A-Z]{3}[0-9]{6}$')
    ) STORED,
    industry_division VARCHAR(255) NULL,
    industry_sector VARCHAR(100) NULL,
    industry_source VARCHAR(30) NULL,
    latitude DOUBLE PRECISION NULL,
    longitude DOUBLE PRECISION NULL,
    geocode_status VARCHAR(50) NOT NULL DEFAULT 'PENDING',
    resolved_state VARCHAR(100) NULL,
    resolved_city VARCHAR(100) NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

   CONSTRAINT no_overlapping_filings EXCLUDE USING gist (
        corporate_id_number WITH =,
        (COALESCE(reporting_boundary, 'Standalone')) WITH =,
        (daterange(financial_year_start, financial_year_end, '[]')) WITH &&
    ) WHERE (financial_year_start IS NOT NULL AND financial_year_end IS NOT NULL AND financial_year_start <= financial_year_end)
);

-- index
CREATE INDEX idx_entity_cin ON entity_list (corporate_id_number);
CREATE INDEX idx_entity_isin ON entity_list (isin);
CREATE INDEX idx_entity_nse_symbol ON entity_list (UPPER(nse_symbol));
CREATE INDEX idx_entity_bse_scrip ON entity_list (bse_scrip_code);
CREATE INDEX idx_entity_file_uuid ON entity_list (file_uuid);
CREATE INDEX idx_entity_sector ON entity_list (industry_sector);
CREATE INDEX idx_entity_industry_source ON entity_list (industry_source);
CREATE INDEX idx_entity_coords ON entity_list (latitude, longitude);
CREATE INDEX idx_entity_geocode_status ON entity_list (geocode_status);

-- comments 36
COMMENT ON TABLE entity_list IS 'Horizontal master record of corporate profiles, financial periods, and exchange-verified industry classifications';
COMMENT ON COLUMN entity_list.entity_id IS 'Unique UUID identifier for the company filing record';
COMMENT ON COLUMN entity_list.section IS 'BRSR Section identifier (Default: A)';
COMMENT ON COLUMN entity_list.principle_number IS 'Principle number (Default: 0 for Section A)';
COMMENT ON COLUMN entity_list.sub_section IS 'Sub-section identifier (Default: 1 for General Disclosures)';
COMMENT ON COLUMN entity_list.friendly_group IS 'Grouping identifier (Default: entity_list)';
COMMENT ON COLUMN entity_list.stock_market IS 'Exchange source where filing was retrieved (BSE or NSE)';
COMMENT ON COLUMN entity_list.file_uuid IS 'Foreign key referencing file_list(file_id) to trace the source filing file';
COMMENT ON COLUMN entity_list.corporate_id_number IS 'Corporate Identity Number (CIN) issued by MCA or filing identifier';
COMMENT ON COLUMN entity_list.isin IS 'International Securities Identification Number (12-char stock identifier)';
COMMENT ON COLUMN entity_list.nse_symbol IS 'NSE Ticker Symbol associated with the filing';
COMMENT ON COLUMN entity_list.bse_scrip_code IS 'BSE Scrip Code associated with the filing';
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
COMMENT ON COLUMN entity_list.nic_code IS 'Validated 5-digit NIC code extracted strictly from structurally valid 21-char MCA CINs (NULL otherwise)';
COMMENT ON COLUMN entity_list.industry_division IS 'SEBI/Exchange detailed industry division';
COMMENT ON COLUMN entity_list.industry_sector IS 'Peer group category used for cross-company ESG analytics and intensity benchmarking';
COMMENT ON COLUMN entity_list.industry_source IS 'Audit source showing how industry was resolved (EXCHANGE_ISIN, EXCHANGE_TICKER, EXCHANGE_CIN, EXCHANGE_NAME, NIC_5DIGIT, NIC_2DIGIT, STATUTORY_KEYWORD, UNCLASSIFIED)';
COMMENT ON COLUMN entity_list.latitude IS 'Geocoded latitude coordinate of registered or corporate office';
COMMENT ON COLUMN entity_list.longitude IS 'Geocoded longitude coordinate of registered or corporate office';
COMMENT ON COLUMN entity_list.geocode_status IS 'Audit status: PENDING, RESOLVED, NOT_FOUND, NO_PINCODE, or NO_ADDRESS';
COMMENT ON COLUMN entity_list.resolved_state IS 'Normalized State or Union Territory resolved from geocoding lookup';
COMMENT ON COLUMN entity_list.resolved_city IS 'Normalized City, Town, or District resolved from geocoding lookup';
COMMENT ON COLUMN entity_list.created_at IS 'Timestamp when the entity master record was inserted';

-- function - Multi-Key Deterministic Industry Resolution (Exchange Master -> NIC 5/2-Digit -> Statutory Fallback)
CREATE OR REPLACE FUNCTION trg_enrich_entity_industry()
RETURNS TRIGGER AS $$
DECLARE
    clean_cin VARCHAR(21);
    clean_nic VARCHAR(5);
    clean_div VARCHAR(2);
    clean_isin VARCHAR(12);
    clean_sym VARCHAR(50);
    clean_scrip VARCHAR(20);
    clean_name VARCHAR(255);
    upper_name VARCHAR(255);
    ex RECORD;
    im RECORD;
BEGIN
    clean_isin  := NULLIF(UPPER(TRIM(NEW.isin)), '');
    clean_sym   := NULLIF(UPPER(TRIM(NEW.nse_symbol)), '');
    clean_scrip := NULLIF(TRIM(NEW.bse_scrip_code), '');
    clean_cin   := substring(UPPER(TRIM(COALESCE(NEW.corporate_id_number, '')))
                   FROM '([LU][0-9]{5}[A-Z]{2}[0-9]{4}[A-Z]{3}[0-9]{6})');
    upper_name  := UPPER(TRIM(COALESCE(NEW.company_name, '')));

    -- Tier 1: Authoritative ISIN lookup against exchange_industry_master
    IF clean_isin IS NOT NULL THEN
        SELECT sector, industry
        INTO ex
        FROM exchange_industry_master
        WHERE isin = clean_isin
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := ex.sector;
            NEW.industry_division := ex.industry;
            NEW.industry_source := 'EXCHANGE_ISIN';
            RETURN NEW;
        END IF;
    END IF;

    -- Tier 2a: Official NSE Symbol
    IF clean_sym IS NOT NULL THEN
        SELECT sector, industry
        INTO ex
        FROM exchange_industry_master
        WHERE UPPER(symbol) = clean_sym
        ORDER BY updated_at DESC
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := ex.sector;
            NEW.industry_division := ex.industry;
            NEW.industry_source := 'EXCHANGE_TICKER';
            RETURN NEW;
        END IF;
    END IF;

    -- Tier 2b: Official BSE Scrip Code
    IF clean_scrip IS NOT NULL THEN
        SELECT sector, industry
        INTO ex
        FROM exchange_industry_master
        WHERE scrip_code = clean_scrip
        ORDER BY updated_at DESC
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := ex.sector;
            NEW.industry_division := ex.industry;
            NEW.industry_source := 'EXCHANGE_TICKER';
            RETURN NEW;
        END IF;
    END IF;

    -- Tier 3: Validated 21-char CIN match against exchange_industry_master
    IF clean_cin IS NOT NULL THEN
        SELECT sector, industry
        INTO ex
        FROM exchange_industry_master
        WHERE cin = clean_cin
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := ex.sector;
            NEW.industry_division := ex.industry;
            NEW.industry_source := 'EXCHANGE_CIN';
            RETURN NEW;
        END IF;
    END IF;

    -- Tier 4: Normalized Company Name Fuzzy Match (pg_trgm >= 0.85)
    IF upper_name <> '' THEN
        clean_name := normalize_company_name(NEW.company_name);

        IF clean_name IS NOT NULL AND LENGTH(clean_name) >= 4 THEN
            SELECT sector, industry
            INTO ex
            FROM exchange_industry_master
            WHERE normalized_name % clean_name
              AND similarity(normalized_name, clean_name) >= 0.85
            ORDER BY similarity(normalized_name, clean_name) DESC
            LIMIT 1;

            IF FOUND THEN
                NEW.industry_sector := ex.sector;
                NEW.industry_division := ex.industry;
                NEW.industry_source := 'EXCHANGE_NAME';
                RETURN NEW;
            END IF;
        END IF;
    END IF;

    -- Tier 5: Deterministic NIC Code Fallback from MCA CIN using industry_master
    IF clean_cin IS NOT NULL THEN
        clean_nic := substring(clean_cin FROM '^[LU]([0-9]{5})');
        clean_div := substring(clean_cin FROM '^[LU]([0-9]{2})');

        -- Tier 5a: Exact 5-digit NIC match (handles legacy NIC-2004 codes accurately)
        SELECT esg_sector, division_name
        INTO im
        FROM industry_master
        WHERE nic_code = clean_nic
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := im.esg_sector;
            NEW.industry_division := im.division_name;
            NEW.industry_source := 'NIC_5DIGIT';
            RETURN NEW;
        END IF;

        -- Tier 5b: 2-digit MOSPI Division match
        SELECT esg_sector, division_name
        INTO im
        FROM industry_master
        WHERE division_code = clean_div
        ORDER BY LENGTH(nic_code) ASC
        LIMIT 1;

        IF FOUND THEN
            NEW.industry_sector := im.esg_sector;
            NEW.industry_division := im.division_name;
            NEW.industry_source := 'NIC_2DIGIT';
            RETURN NEW;
        END IF;
    END IF;

    -- Tier 6: Statutory / Non-MCA Entities (e.g., SBI, LIC, PSBs without 21-char MCA CIN)
    IF upper_name <> '' THEN
        IF upper_name ~ '\m(BANK|FINANCE|FINANCIAL|FINSERV|INSURANCE|ASSURANCE|SECURITIES|MUTUAL FUND|CAPITAL)\M' THEN
            NEW.industry_division := 'Banking, Financial Services & Insurance';
            NEW.industry_sector := 'Banking & Financial Services';
            NEW.industry_source := 'STATUTORY_KEYWORD';
            RETURN NEW;
        ELSIF upper_name ~ '\m(POWER|ELECTRIC|ENERGY|NTPC|NHPC|GRID)\M' THEN
            NEW.industry_division := 'Electricity, gas, steam and air conditioning supply';
            NEW.industry_sector := 'Power & Utilities';
            NEW.industry_source := 'STATUTORY_KEYWORD';
            RETURN NEW;
        ELSIF upper_name ~ '\m(PETROLEUM|OIL|GAS|REFINERY|ONGC|GAIL|IOCL|BPCL|HPCL)\M' THEN
            NEW.industry_division := 'Oil, Gas & Refining';
            NEW.industry_sector := 'Oil, Gas & Refining';
            NEW.industry_source := 'STATUTORY_KEYWORD';
            RETURN NEW;
        ELSIF upper_name ~ '\m(STEEL|ALUMINIUM|MINING|MINERAL|COAL|COPPER|ZINC)\M' THEN
            NEW.industry_division := 'Metals & Mining';
            NEW.industry_sector := 'Metals & Mining (Steel/Aluminium)';
            NEW.industry_source := 'STATUTORY_KEYWORD';
            RETURN NEW;
        END IF;
    END IF;

    -- Final fallback only when no identifier or pattern matched
    NEW.industry_division := 'Pending Exchange Mapping';
    NEW.industry_sector := 'Unclassified';
    NEW.industry_source := 'UNCLASSIFIED';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION sync_entity_industries()
RETURNS INTEGER AS $$
DECLARE
    updated_count INTEGER;
BEGIN
    UPDATE entity_list
    SET corporate_id_number = corporate_id_number
    WHERE industry_source IS NULL
       OR industry_source NOT IN ('EXCHANGE_ISIN', 'EXCHANGE_TICKER', 'EXCHANGE_CIN', 'EXCHANGE_NAME')
       OR industry_sector = 'Unclassified';

    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RETURN updated_count;
END;
$$ LANGUAGE plpgsql;

-- trigger
CREATE OR REPLACE TRIGGER trg_set_industry
BEFORE INSERT OR UPDATE OF corporate_id_number, isin, nse_symbol, bse_scrip_code, company_name
ON entity_list
FOR EACH ROW
EXECUTE FUNCTION trg_enrich_entity_industry();

CREATE OR REPLACE TRIGGER trg_set_coordinates
BEFORE INSERT OR UPDATE OF address_registered_office, address_corporate_office
ON entity_list
FOR EACH ROW
EXECUTE FUNCTION trg_resolve_entity_coordinates();