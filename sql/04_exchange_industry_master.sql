-- function - Shared deterministic company name normalizer for fuzzy trigram matching
CREATE OR REPLACE FUNCTION normalize_company_name(raw_name TEXT)
RETURNS VARCHAR(255) AS $$
    SELECT NULLIF(TRIM(REGEXP_REPLACE(
        REGEXP_REPLACE(
            UPPER(COALESCE(raw_name, '')),
            '\m(LIMITED|LTD|PRIVATE|PVT|CO|CORP|CORPORATION)\M', '', 'g'
        ),
        '[^A-Z0-9]+', ' ', 'g'
    )), '')::VARCHAR(255);
$$ LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE;

-- create table - Verified BSE & NSE Industry Classification Master (Multi-Key: ISIN, Symbol, Scrip, CIN)
CREATE TABLE exchange_industry_master (
    master_id UUID PRIMARY KEY DEFAULT uuidv7(),
    isin VARCHAR(12) NULL UNIQUE,
    cin VARCHAR(21) NULL UNIQUE,
    symbol VARCHAR(50) NULL,
    scrip_code VARCHAR(20) NULL,
    company_name VARCHAR(255) NOT NULL,
    normalized_name VARCHAR(255) GENERATED ALWAYS AS (normalize_company_name(company_name)) STORED,
    macro_sector VARCHAR(100) NOT NULL,
    sector VARCHAR(100) NOT NULL,
    industry VARCHAR(120) NOT NULL,
    basic_industry VARCHAR(120) NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- index (isin and cin already have unique B-tree indexes from UNIQUE constraints)
CREATE INDEX idx_ex_ind_symbol ON exchange_industry_master (UPPER(symbol));
CREATE INDEX idx_ex_ind_scrip ON exchange_industry_master (scrip_code);
CREATE INDEX idx_ex_ind_sector ON exchange_industry_master (sector);
CREATE INDEX idx_ex_ind_norm_name_trgm ON exchange_industry_master USING gin (normalized_name gin_trgm_ops);

-- comments 13
COMMENT ON TABLE exchange_industry_master IS 'Official SEBI 4-tier BSE/NSE industry taxonomy mapped by ISIN, Symbol, Scrip Code, and CIN';
COMMENT ON COLUMN exchange_industry_master.master_id IS 'Unique UUID primary key for the exchange industry mapping record';
COMMENT ON COLUMN exchange_industry_master.isin IS 'International Securities Identification Number (12-char primary stock identifier, e.g. INE002A01018)';
COMMENT ON COLUMN exchange_industry_master.cin IS 'Corporate Identity Number (21-char MCA identifier, nullable for statutory/bank entities)';
COMMENT ON COLUMN exchange_industry_master.symbol IS 'NSE Ticker Symbol (e.g. TATASTEEL, RELIANCE)';
COMMENT ON COLUMN exchange_industry_master.scrip_code IS 'BSE Scrip Code (e.g. 500470, 500325)';
COMMENT ON COLUMN exchange_industry_master.company_name IS 'Official listed entity name registered on the exchange';
COMMENT ON COLUMN exchange_industry_master.normalized_name IS 'Pre-computed stripped uppercase name for fast trigram fuzzy matching';
COMMENT ON COLUMN exchange_industry_master.macro_sector IS 'SEBI Tier-1 Macro-Economic Sector (e.g. Energy, Financial Services, Information Technology)';
COMMENT ON COLUMN exchange_industry_master.sector IS 'SEBI Tier-2 Sector used as the primary ESG benchmarking peer group';
COMMENT ON COLUMN exchange_industry_master.industry IS 'SEBI Tier-3 Industry Division';
COMMENT ON COLUMN exchange_industry_master.basic_industry IS 'SEBI Tier-4 Basic Industry granular classification';
COMMENT ON COLUMN exchange_industry_master.updated_at IS 'Timestamp when the exchange taxonomy record was last synced';