-- ============================================================================
-- 0. TEARDOWN (Drop existing tables in reverse dependency order)
-- ============================================================================
DROP TABLE IF EXISTS brsr_facts CASCADE;
DROP TABLE IF EXISTS entity_list CASCADE;
DROP TABLE IF EXISTS file_list CASCADE;
DROP TABLE IF EXISTS form_master CASCADE;
DROP TABLE IF EXISTS unit_conversions CASCADE;
DROP TABLE IF EXISTS industry_master CASCADE;

-- ============================================================================
-- EXTENSIONS
-- ============================================================================
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "btree_gist";  -- Required for temporal non-overlapping ranges
CREATE EXTENSION IF NOT EXISTS "vector";      -- Ready for Phase 5 RAG embeddings

-- ============================================================================
-- 1. FILE_LIST (Download & Ingestion Tracker)
-- ============================================================================
CREATE TABLE file_list (
    file_id UUID PRIMARY KEY DEFAULT uuidv7(),
    file_name VARCHAR(255) NOT NULL UNIQUE,
    stock_market VARCHAR(50) NOT NULL, -- 'BSE' or 'NSE'
    upload_date DATE NOT NULL,
    upload_time TIME WITHOUT TIME ZONE NOT NULL,
    file_type VARCHAR(50) NOT NULL,    -- 'PDF' or 'XML'
    mapped BOOLEAN NOT NULL DEFAULT FALSE,
    unmapped_reason VARCHAR(255) NULL,  -- e.g., 'New Questions', 'Corrupt XML', 'Download Timeout'
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX idx_file_list_lookup
ON file_list (file_name, mapped);

CREATE INDEX idx_file_list_market_date
ON file_list (stock_market, upload_date DESC);

-- ============================================================================
-- 2. FORM_MASTER (Question Dictionary & Taxonomy Rules)
-- ============================================================================
CREATE TABLE form_master (
    form_id UUID PRIMARY KEY DEFAULT uuidv7(),
    question_key VARCHAR(50) NOT NULL,
    section VARCHAR(10) NOT NULL,                     -- 'A', 'B', 'C'
    principle_number SMALLINT NOT NULL DEFAULT 0,     -- 0 to 9
    sub_section SMALLINT NOT NULL DEFAULT 0,          -- 1 to 7
    indicator_type VARCHAR(20) NOT NULL,              -- 'Essential', 'Leadership', 'General'
    question_in_form SMALLINT NULL,
    question_number SMALLINT NOT NULL,
    followup_question_first VARCHAR(20) NOT NULL,
    followup_question_second VARCHAR(20) NULL,
    question_order SMALLINT NOT NULL,
    question_type VARCHAR(20) NOT NULL,
    response_datatype VARCHAR(50) NOT NULL,           -- 'number', 'string', 'date', 'boolean', 'email', 'phone', 'hyperlink'
    response_required BOOLEAN NOT NULL DEFAULT FALSE,
    calculation BOOLEAN NOT NULL DEFAULT FALSE,
    contextref_increment BOOLEAN NOT NULL DEFAULT FALSE, -- Multi-item dynamic axis flag
    visibility_dependence VARCHAR(50) NOT NULL DEFAULT '0',
    visibility_trigger VARCHAR(255) NULL,
    total_field BOOLEAN NOT NULL DEFAULT FALSE,
    friendly_grouping VARCHAR(255) NULL,
    field_flag SMALLINT NULL,                         -- 0 = Fixed, 1 = Multi-Column, 2 = 9-Principle Grid
    anchor_reference SMALLINT NULL,
    append_to_field VARCHAR(20) NULL,
    legacy BOOLEAN NOT NULL DEFAULT FALSE,
    section_text VARCHAR(255) NULL,
    principle_text VARCHAR(255) NULL,
    sub_section_text VARCHAR(255) NULL,
    indicator_text VARCHAR(255) NULL,
    question_text TEXT NOT NULL,
    indicator_capmkt TEXT NULL,                       -- Raw XML tag name
    context_reference TEXT NULL,                      -- Raw context ID (e.g., 'DCYMain')
    standard_indicator_capmkt TEXT NULL,              -- Canonical tag name
    standard_context_reference TEXT NULL,             -- Canonical context name
    response_options TEXT NULL,
    input_text TEXT NULL,
    formula TEXT NULL,
    comments TEXT NULL
);

CREATE INDEX idx_form_master_join_keys
ON form_master (indicator_capmkt, context_reference);

CREATE INDEX idx_form_master_standard_keys
ON form_master (standard_indicator_capmkt, standard_context_reference);

CREATE INDEX idx_form_master_grouping
ON form_master (friendly_grouping);

-- ============================================================================
-- 3. ENTITY_LIST (Company Master Disclosures & Temporal Boundaries)
-- ============================================================================
CREATE TABLE entity_list (
    entity_id UUID PRIMARY KEY DEFAULT uuidv7(),
    section VARCHAR(10) NOT NULL DEFAULT 'A',
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 1,
    friendly_group VARCHAR(255) NOT NULL DEFAULT 'entity_list',
    stock_market VARCHAR(50) NOT NULL,
    file_uuid UUID NOT NULL REFERENCES file_list(file_id) ON DELETE CASCADE,
    corporate_id_number VARCHAR(25) NOT NULL,         -- CIN
    company_name VARCHAR(255) NOT NULL,
    address_corporate_office TEXT NULL,
    address_registered_office TEXT NULL,
    contact_person_name VARCHAR(500) NULL,
    contact_person_phone VARCHAR(255) NULL,
    contact_person_email VARCHAR(255) NULL,
    company_email VARCHAR(255) NULL,
    company_telephone VARCHAR(255) NULL,
    website VARCHAR(255) NULL,
    reporting_boundary VARCHAR(50) NULL DEFAULT 'Standalone', -- 'Standalone' or 'Consolidated'
    financial_year_start DATE NULL,
    financial_year_end DATE NULL,
    incorporation_date DATE NULL,
    paid_up_shares_value NUMERIC(20, 4) NULL,
    nic_code VARCHAR(10) NULL,                        -- Classified 5-digit/2-digit NIC
    industry_division VARCHAR(120) NULL,              -- Macro economic division title
    industry_sector VARCHAR(80) NULL,                 -- ESG benchmarking peer group
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    -- Temporal Exclusion: Prevents duplicate filings for the exact same reporting cycle
    CONSTRAINT no_overlapping_filings EXCLUDE USING gist (
        corporate_id_number WITH =,
        COALESCE(reporting_boundary, 'Standalone') WITH =,
        daterange(financial_year_start, financial_year_end, '[]') WITH &&
    ) WHERE (financial_year_start IS NOT NULL AND financial_year_end IS NOT NULL)
);

CREATE INDEX idx_entity_cin
ON entity_list (corporate_id_number);

CREATE INDEX idx_entity_file_uuid
ON entity_list (file_uuid);

CREATE INDEX idx_entity_sector
ON entity_list (industry_sector);

-- ============================================================================
-- 4. BRSR_FACTS (Granular XBRL Facts EAV Table)
-- ============================================================================
CREATE TABLE brsr_facts (
    brsr_fact_id UUID PRIMARY KEY DEFAULT uuidv7(),
    section VARCHAR(10) NOT NULL,
    principle_number SMALLINT NOT NULL DEFAULT 0,
    indicator_type VARCHAR(20) NOT NULL,
    sub_section SMALLINT NOT NULL DEFAULT 0,
    client_id VARCHAR(100) NOT NULL,                  -- CIN
    file_uuid UUID NOT NULL REFERENCES file_list(file_id) ON DELETE CASCADE,
    client_uuid UUID NOT NULL REFERENCES entity_list(entity_id) ON DELETE CASCADE,
    indicator_capmkt TEXT NOT NULL,
    standard_indicator_capmkt TEXT NOT NULL,
    context_reference TEXT NOT NULL,
    standard_context_reference TEXT NOT NULL,
    friendly_grouping VARCHAR(255) NULL,
    response_datatype VARCHAR(50) NOT NULL,           -- 'number' or 'string'
    raw_value TEXT NULL,
    numeric_value NUMERIC(20, 4) NULL,
    unit_reference VARCHAR(50) NULL,
    standard_unit_reference VARCHAR(50) NULL,
    decimals SMALLINT NULL,
    standard_raw_value TEXT NULL,
    standard_numeric_value NUMERIC(20, 4) NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- PG 18 Skip-Scan Composite Index:
CREATE INDEX idx_facts_skip_scan
ON brsr_facts (friendly_grouping, client_uuid, standard_indicator_capmkt);

CREATE INDEX idx_facts_file_uuid
ON brsr_facts (file_uuid);

CREATE INDEX idx_facts_client_uuid
ON brsr_facts (client_uuid);

CREATE INDEX idx_facts_standard_concept
ON brsr_facts (standard_indicator_capmkt, standard_context_reference);

CREATE INDEX idx_facts_section_principle
ON brsr_facts (section, principle_number);

-- ============================================================================
-- 5. UNIT_CONVERSIONS (Normalization Multipliers)
-- ============================================================================
CREATE TABLE unit_conversions (
    conversion_id UUID PRIMARY KEY DEFAULT uuidv7(),
    metric_category VARCHAR(50) NOT NULL,       -- 'Energy', 'Volume', 'Mass', 'GHG', 'Concentration'
    raw_unit VARCHAR(50) NOT NULL UNIQUE,       -- Normalized upper-case key (e.g., 'MEGAJOULE', 'MJ', 'TONNES')
    canonical_unit VARCHAR(50) NOT NULL,        -- Target canonical label (e.g., 'GJ', 't', 'KL', 'GtCO2e')
    conversion_factor DOUBLE PRECISION NOT NULL,-- Multiplier (e.g., 0.001, 1000.0, 1e-9)
    notes TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX idx_unit_raw_lookup ON unit_conversions (raw_unit);

INSERT INTO unit_conversions (metric_category, raw_unit, canonical_unit, conversion_factor, notes)
VALUES
    -- Energy -> Gigajoules (GJ)
    ('Energy', 'GJ', 'GJ', 1.0, 'Base canonical unit for energy'),
    ('Energy', 'GIGAJOULE', 'GJ', 1.0, 'Alternative standard label'),
    ('Energy', 'MJ', 'GJ', 0.001, '1 Megajoule = 0.001 GJ'),
    ('Energy', 'MEGAJOULE', 'GJ', 0.001, 'Megajoule full word'),
    ('Energy', 'MWH', 'GJ', 3.6, '1 Megawatt hour = 3.6 GJ'),
    ('Energy', 'MEGAWATTHOUR', 'GJ', 3.6, 'Megawatt hour full word'),
    ('Energy', 'KWH', 'GJ', 0.0036, '1 Kilowatt hour = 0.0036 GJ'),
    ('Energy', 'KILOWATTHOUR', 'GJ', 0.0036, 'Kilowatt hour full word'),
    ('Energy', 'TOE', 'GJ', 41.868, '1 Tonne of Oil Equivalent = 41.868 GJ'),
    ('Energy', 'TONNESOFOILEQUIVALENT', 'GJ', 41.868, 'TOE full word'),
    ('Energy', 'GCAL', 'GJ', 4.184, '1 Gigacalorie = 4.184 GJ'),
    ('Energy', 'GIGACALORIE', 'GJ', 4.184, 'Gigacalorie full word'),

    -- Mass -> Metric Tonnes (t)
    ('Mass', 'T', 't', 1.0, 'Base canonical unit for mass'),
    ('Mass', 'TONNE', 't', 1.0, 'Metric Tonne alternative'),
    ('Mass', 'METRICTONNE', 't', 1.0, 'Metric Tonne full word'),
    ('Mass', 'KG', 't', 0.001, '1 Kilogram = 0.001 Tonne'),
    ('Mass', 'KILOGRAM', 't', 0.001, 'Kilogram full word'),
    ('Mass', 'G', 't', 1e-6, '1 Gram = 0.000001 Tonne'),
    ('Mass', 'GRAM', 't', 1e-6, 'Gram full word'),
    ('Mass', 'MG', 't', 1e-9, '1 Milligram = 1e-9 Tonne'),
    ('Mass', 'QUINTAL', 't', 0.1, '1 Quintal = 0.1 Tonne'),
    ('Mass', 'LBS', 't', 0.00045359237, '1 Pound = 0.00045359237 Tonne'),

    -- Volume -> Kiloliters (KL)
    ('Volume', 'KL', 'KL', 1.0, 'Base canonical unit for volume'),
    ('Volume', 'KILOLITER', 'KL', 1.0, 'Kiloliter alternative'),
    ('Volume', 'KILOLITRE', 'KL', 1.0, 'Kilolitre UK spelling'),
    ('Volume', 'M3', 'KL', 1.0, '1 Cubic meter = 1 Kiloliter'),
    ('Volume', 'CUBICMETER', 'KL', 1.0, 'Cubic meter full word'),
    ('Volume', 'L', 'KL', 0.001, '1 Liter = 0.001 Kiloliter'),
    ('Volume', 'LITER', 'KL', 0.001, 'Liter full word'),
    ('Volume', 'LITRE', 'KL', 0.001, 'Litre UK spelling'),
    ('Volume', 'ML', 'KL', 1000.0, '1 Million Liters = 1000 Kiloliters'),
    ('Volume', 'MILLIONLITER', 'KL', 1000.0, 'Million liter full word'),
    ('Volume', 'GAL', 'KL', 0.00378541, '1 US Gallon = 0.00378541 Kiloliters'),

    -- GHG -> Metric Tonnes CO2 equivalent (t)
    ('GHG', 'TCO2E', 't', 1.0, 'Tonne of CO2 equivalent'),
    ('GHG', 'TONNECO2E', 't', 1.0, 'Tonne CO2 equivalent full word'),
    ('GHG', 'KGCO2E', 't', 0.001, '1 Kilogram of CO2 equivalent = 0.001 Tonne'),
    ('GHG', 'GTCO2E', 't', 1e9, '1 Gigatonne CO2 equivalent = 1e9 Tonnes')
ON CONFLICT (raw_unit) DO NOTHING;

-- ============================================================================
-- 6. INDUSTRY_MASTER (NIC Classification & Peer Groups)
-- ============================================================================
CREATE TABLE industry_master (
    industry_id UUID PRIMARY KEY DEFAULT uuidv7(),
    nic_code VARCHAR(10) NOT NULL UNIQUE,
    division_code VARCHAR(5) NOT NULL,
    division_name VARCHAR(255) NOT NULL,
    section_code VARCHAR(2) NOT NULL,
    section_name VARCHAR(255) NOT NULL,
    esg_sector VARCHAR(100) NOT NULL
);

CREATE INDEX idx_industry_division ON industry_master(division_code);

INSERT INTO industry_master (nic_code, division_code, division_name, section_code, section_name, esg_sector)
VALUES
    ('01', '01', 'Crop and animal production, hunting and related service activities', 'A', 'Agriculture, Forestry and Fishing', 'Agriculture & Agro-commodities'),
    ('02', '02', 'Forestry and logging', 'A', 'Agriculture, Forestry and Fishing', 'Paper, Packaging & Forest Products'),
    ('03', '03', 'Fishing and aquaculture', 'A', 'Agriculture, Forestry and Fishing', 'Agriculture & Agro-commodities'),
    ('05', '05', 'Mining of coal and lignite', 'B', 'Mining and Quarrying', 'Mining & Natural Resources'),
    ('06', '06', 'Extraction of crude petroleum and natural gas', 'B', 'Mining and Quarrying', 'Oil, Gas & Refining'),
    ('07', '07', 'Mining of metal ores', 'B', 'Mining and Quarrying', 'Metals & Mining (Steel/Aluminium)'),
    ('08', '08', 'Other mining and quarrying', 'B', 'Mining and Quarrying', 'Mining & Natural Resources'),
    ('09', '09', 'Mining support service activities', 'B', 'Mining and Quarrying', 'Mining & Natural Resources'),
    ('10', '10', 'Manufacture of food products', 'C', 'Manufacturing', 'Consumer Staples & FMCG'),
    ('11', '11', 'Manufacture of beverages', 'C', 'Manufacturing', 'Consumer Staples & FMCG'),
    ('12', '12', 'Manufacture of tobacco products', 'C', 'Manufacturing', 'Consumer Staples & FMCG'),
    ('13', '13', 'Manufacture of textiles', 'C', 'Manufacturing', 'Textiles, Apparel & Leather'),
    ('14', '14', 'Manufacture of wearing apparel', 'C', 'Manufacturing', 'Textiles, Apparel & Leather'),
    ('15', '15', 'Manufacture of leather and related products', 'C', 'Manufacturing', 'Textiles, Apparel & Leather'),
    ('16', '16', 'Manufacture of wood and products of wood and cork', 'C', 'Manufacturing', 'Paper, Packaging & Forest Products'),
    ('17', '17', 'Manufacture of paper and paper products', 'C', 'Manufacturing', 'Paper, Packaging & Forest Products'),
    ('18', '18', 'Printing and reproduction of recorded media', 'C', 'Manufacturing', 'Media, Entertainment & Publishing'),
    ('19', '19', 'Manufacture of coke and refined petroleum products', 'C', 'Manufacturing', 'Oil, Gas & Refining'),
    ('20', '20', 'Manufacture of chemicals and chemical products', 'C', 'Manufacturing', 'Chemicals & Petrochemicals'),
    ('21', '21', 'Manufacture of pharmaceuticals, medicinal chemical and botanical products', 'C', 'Manufacturing', 'Pharmaceuticals & Healthcare'),
    ('22', '22', 'Manufacture of rubber and plastics products', 'C', 'Manufacturing', 'Chemicals & Plastics'),
    ('23', '23', 'Manufacture of other non-metallic mineral products (Cement/Ceramics)', 'C', 'Manufacturing', 'Cement & Building Materials'),
    ('24', '24', 'Manufacture of basic metals (Steel, Aluminium, Copper)', 'C', 'Manufacturing', 'Metals & Mining (Steel/Aluminium)'),
    ('25', '25', 'Manufacture of fabricated metal products', 'C', 'Manufacturing', 'Capital Goods & Engineering'),
    ('26', '26', 'Manufacture of computer, electronic and optical products', 'C', 'Manufacturing', 'Technology Hardware & Electronics'),
    ('27', '27', 'Manufacture of electrical equipment', 'C', 'Manufacturing', 'Capital Goods & Engineering'),
    ('28', '28', 'Manufacture of machinery and equipment n.e.c.', 'C', 'Manufacturing', 'Capital Goods & Engineering'),
    ('29', '29', 'Manufacture of motor vehicles, trailers and semi-trailers', 'C', 'Manufacturing', 'Automotive & Auto Components'),
    ('30', '30', 'Manufacture of other transport equipment', 'C', 'Manufacturing', 'Automotive & Auto Components'),
    ('31', '31', 'Manufacture of furniture', 'C', 'Manufacturing', 'Consumer Discretionary'),
    ('32', '32', 'Other manufacturing (Gems, Jewellery, Sports Goods)', 'C', 'Manufacturing', 'Consumer Discretionary'),
    ('33', '33', 'Repair and installation of machinery and equipment', 'C', 'Manufacturing', 'Capital Goods & Engineering'),
    ('35', '35', 'Electricity, gas, steam and air conditioning supply', 'D', 'Electricity, Gas, Steam and Air Conditioning', 'Power & Utilities'),
    ('36', '36', 'Water collection, treatment and supply', 'E', 'Water Supply; Sewerage, Waste Management', 'Utilities & Waste Management'),
    ('37', '37', 'Sewerage', 'E', 'Water Supply; Sewerage, Waste Management', 'Utilities & Waste Management'),
    ('38', '38', 'Waste collection, treatment and disposal activities', 'E', 'Water Supply; Sewerage, Waste Management', 'Utilities & Waste Management'),
    ('39', '39', 'Remediation activities and other waste management services', 'E', 'Water Supply; Sewerage, Waste Management', 'Utilities & Waste Management'),
    ('41', '41', 'Construction of buildings', 'F', 'Construction', 'Infrastructure & Construction'),
    ('42', '42', 'Civil engineering', 'F', 'Construction', 'Infrastructure & Construction'),
    ('43', '43', 'Specialized construction activities', 'F', 'Construction', 'Infrastructure & Construction'),
    ('45', '45', 'Wholesale and retail trade and repair of motor vehicles', 'G', 'Wholesale and Retail Trade', 'Retail & Wholesale Trade'),
    ('46', '46', 'Wholesale trade, except of motor vehicles and motorcycles', 'G', 'Wholesale and Retail Trade', 'Retail & Wholesale Trade'),
    ('47', '47', 'Retail trade, except of motor vehicles and motorcycles', 'G', 'Wholesale and Retail Trade', 'Retail & Wholesale Trade'),
    ('49', '49', 'Land transport and transport via pipelines', 'H', 'Transportation and Storage', 'Transportation & Logistics'),
    ('50', '50', 'Water transport', 'H', 'Transportation and Storage', 'Transportation & Logistics'),
    ('51', '51', 'Air transport', 'H', 'Transportation and Storage', 'Transportation & Logistics'),
    ('52', '52', 'Warehousing and support activities for transportation', 'H', 'Transportation and Storage', 'Transportation & Logistics'),
    ('53', '53', 'Postal and courier activities', 'H', 'Transportation and Storage', 'Transportation & Logistics'),
    ('55', '55', 'Accommodation (Hotels & Resorts)', 'I', 'Accommodation and Food Service Activities', 'Hospitality & Tourism'),
    ('56', '56', 'Food and beverage service activities (Restaurants)', 'I', 'Accommodation and Food Service Activities', 'Hospitality & Tourism'),
    ('58', '58', 'Publishing activities (Software/Books)', 'J', 'Information and Communication', 'Information Technology'),
    ('61', '61', 'Telecommunications', 'J', 'Information and Communication', 'Telecommunications'),
    ('62', '62', 'Computer programming, consultancy and related activities', 'J', 'Information and Communication', 'Information Technology'),
    ('63', '63', 'Information service activities (Data Processing, Portals)', 'J', 'Information and Communication', 'Information Technology'),
    ('64', '64', 'Financial service activities, except insurance and pension', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('65', '65', 'Insurance, reinsurance and pension funding', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('66', '66', 'Other financial activities (Stock Exchanges, Broking)', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('68', '68', 'Real estate activities', 'L', 'Real Estate Activities', 'Real Estate & Urban Development'),
    ('69', '69', 'Legal and accounting activities', 'M', 'Professional, Scientific and Technical Activities', 'Professional Services'),
    ('70', '70', 'Activities of head offices; management consultancy', 'M', 'Professional, Scientific and Technical Activities', 'Professional Services'),
    ('71', '71', 'Architectural and engineering activities; technical testing', 'M', 'Professional, Scientific and Technical Activities', 'Professional Services'),
    ('72', '72', 'Scientific research and development', 'M', 'Professional, Scientific and Technical Activities', 'Pharmaceuticals & Healthcare'),
    ('73', '73', 'Advertising and market research', 'M', 'Professional, Scientific and Technical Activities', 'Media, Entertainment & Publishing'),
    ('86', '86', 'Human health activities (Hospitals & Clinics)', 'Q', 'Human Health and Social Work Activities', 'Healthcare & Hospitals'),
    ('87', '87', 'Residential care activities', 'Q', 'Human Health and Social Work Activities', 'Healthcare & Hospitals')
ON CONFLICT (nic_code) DO NOTHING;

INSERT INTO industry_master (nic_code, division_code, division_name, section_code, section_name, esg_sector)
VALUES
    ('62011', '62', 'Writing, modifying, testing of computer program to meet client needs', 'J', 'Information and Communication', 'Information Technology'),
    ('62013', '62', 'Providing software support and maintenance to the clients', 'J', 'Information and Communication', 'Information Technology'),
    ('62099', '62', 'Other information technology and computer service activities n.e.c.', 'J', 'Information and Communication', 'Information Technology'),
    ('72200', '62', 'Software publishing, consultancy and supply (Legacy NIC-2004)', 'J', 'Information and Communication', 'Information Technology'),
    ('85110', '62', 'Data processing, software and computer services (Legacy CIN code)', 'J', 'Information and Communication', 'Information Technology'),
    ('29100', '29', 'Manufacture of motor vehicles', 'C', 'Manufacturing', 'Automotive & Auto Components'),
    ('28920', '29', 'Manufacture of motor vehicles and transport equipment (Legacy code)', 'C', 'Manufacturing', 'Automotive & Auto Components'),
    ('29301', '29', 'Manufacture of motor vehicle parts and accessories', 'C', 'Manufacturing', 'Automotive & Auto Components'),
    ('24101', '24', 'Manufacture of pig iron and spiegeleisen in pigs, blocks or forms', 'C', 'Manufacturing', 'Metals & Mining (Steel/Aluminium)'),
    ('24109', '24', 'Manufacture of other basic iron and steel n.e.c.', 'C', 'Manufacturing', 'Metals & Mining (Steel/Aluminium)'),
    ('27100', '24', 'Manufacture of basic iron and steel (Legacy NIC-2004)', 'C', 'Manufacturing', 'Metals & Mining (Steel/Aluminium)'),
    ('24202', '24', 'Manufacture of aluminium and aluminium products', 'C', 'Manufacturing', 'Metals & Mining (Steel/Aluminium)'),
    ('23941', '23', 'Manufacture of clinkers and cement', 'C', 'Manufacturing', 'Cement & Building Materials'),
    ('26940', '23', 'Manufacture of cement, lime and plaster (Legacy NIC-2004)', 'C', 'Manufacturing', 'Cement & Building Materials'),
    ('21001', '21', 'Manufacture of medicinal active substances for pharmacological properties', 'C', 'Manufacturing', 'Pharmaceuticals & Healthcare'),
    ('21002', '21', 'Manufacture of medicaments (antibiotics, basic vitamins etc.)', 'C', 'Manufacturing', 'Pharmaceuticals & Healthcare'),
    ('24230', '21', 'Manufacture of pharmaceuticals and chemical products (Legacy NIC-2004)', 'C', 'Manufacturing', 'Pharmaceuticals & Healthcare'),
    ('64191', '64', 'Monetary intermediation of commercial banks', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('65110', '64', 'Banking and monetary intermediation (Legacy NIC-2004)', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('64990', '64', 'Other financial service activities, except insurance (NBFCs)', 'K', 'Financial and Insurance Activities', 'Banking & Financial Services'),
    ('19201', '19', 'Production of liquid and gaseous fuels in petroleum refineries', 'C', 'Manufacturing', 'Oil, Gas & Refining'),
    ('23200', '19', 'Manufacture of refined petroleum products (Legacy code)', 'C', 'Manufacturing', 'Oil, Gas & Refining'),
    ('06101', '06', 'Offshore extraction of crude petroleum', 'B', 'Mining and Quarrying', 'Oil, Gas & Refining'),
    ('06102', '06', 'Onshore extraction of crude petroleum', 'B', 'Mining and Quarrying', 'Oil, Gas & Refining')
ON CONFLICT (nic_code) DO NOTHING;

-- ============================================================================
-- DOCUMENTATION & METADATA COMMENTS (Table & Column Level)
-- ============================================================================

-- 1. FILE_LIST COMMENTS
COMMENT ON TABLE file_list IS 'Tracks all downloaded BRSR XML/PDF files, hash states, and ingestion status';
COMMENT ON COLUMN file_list.file_id IS 'Unique UUID primary key for each downloaded file';
COMMENT ON COLUMN file_list.file_name IS 'Downloaded file name (pdf or xml)';
COMMENT ON COLUMN file_list.stock_market IS 'Stock exchange source - BSE or NSE';
COMMENT ON COLUMN file_list.upload_date IS 'Upload date derived from the file name';
COMMENT ON COLUMN file_list.upload_time IS 'Upload time derived from the file name';
COMMENT ON COLUMN file_list.file_type IS 'File format - PDF or XML';
COMMENT ON COLUMN file_list.mapped IS 'Whether data was successfully parsed, mapped, and uploaded into entity_list and brsr_facts';
COMMENT ON COLUMN file_list.unmapped_reason IS 'Diagnostic reason if data failed to map (e.g., New Questions, Corrupt File, Download Timeout, Blank File)';
COMMENT ON COLUMN file_list.created_at IS 'Timestamp when the file record was registered in the database';

-- 2. FORM_MASTER COMMENTS
COMMENT ON TABLE form_master IS 'Master taxonomy dictionary mapping SEBI in-capmkt tags, datatypes, and dynamic axis rules';
COMMENT ON COLUMN form_master.form_id IS 'UUID for each form question entry';
COMMENT ON COLUMN form_master.question_key IS 'NOT UNIQUE - identifier based on section, principle, sub-section/indicator, question number, sub-questions, and order';
COMMENT ON COLUMN form_master.section IS 'BRSR Section - A (General), B (Management), or C (Principles)';
COMMENT ON COLUMN form_master.principle_number IS 'Principle Number - 0 to 9 (Applicable only for Section C; 0 for Sections A and B)';
COMMENT ON COLUMN form_master.sub_section IS 'BRSR Sub-Section - 1 to 7 (Applicable only for Section A)';
COMMENT ON COLUMN form_master.indicator_type IS 'Indicator Type - Essential or Leadership (Applicable to Section C)';
COMMENT ON COLUMN form_master.question_in_form IS 'Question number displayed on the BRSR form. Blanks represent non-questions/gates (e.g., "Whether reasonable assurance undertaken")';
COMMENT ON COLUMN form_master.question_number IS 'Assigned question sequence number covering all rows, including gate conditions';
COMMENT ON COLUMN form_master.followup_question_first IS 'First follow-up question sequence number relative to the main question';
COMMENT ON COLUMN form_master.followup_question_second IS 'Second follow-up question sequence number relative to the main question';
COMMENT ON COLUMN form_master.question_order IS 'Ordering sequence for sub-details requested beneath a primary question (e.g., start date, end date)';
COMMENT ON COLUMN form_master.question_type IS 'Distinguishes between a primary "question" heading and the detailed entry fields beneath it';
COMMENT ON COLUMN form_master.response_datatype IS 'Data type expected for the response: number, string, date, boolean, email, phone, hyperlink';
COMMENT ON COLUMN form_master.response_required IS 'Flags whether a field requires an input from the form framework perspective';
COMMENT ON COLUMN form_master.calculation IS 'Whether the field is a calculated value rather than a manual input';
COMMENT ON COLUMN form_master.contextref_increment IS 'Multi-item dynamic context flag (e.g., D_StockExchangeAxis1, Axis2). 1 signals dynamic cloning during ingestion';
COMMENT ON COLUMN form_master.visibility_dependence IS 'Question sequence that conditionally triggers the visibility of this field';
COMMENT ON COLUMN form_master.visibility_trigger IS 'The specific response value required to show or hide this field';
COMMENT ON COLUMN form_master.total_field IS 'Flags whether the field represents an aggregated sum or total';
COMMENT ON COLUMN form_master.friendly_grouping IS 'Human-readable grouping identifier (e.g., entity_list) used for routing and MCP analytics';
COMMENT ON COLUMN form_master.field_flag IS '0 = Static/Fixed form field; 1 = Multi-column relational table; 2 = 9-Principle horizontal grid (Section B)';
COMMENT ON COLUMN form_master.anchor_reference IS '1 signals that a new relational table row begins; 0 indicates a field belonging to the preceding anchor row';
COMMENT ON COLUMN form_master.append_to_field IS 'Specific suffix or prefix value to append to the field';
COMMENT ON COLUMN form_master.legacy IS 'Flags legacy questions that may not appear in newer taxonomy versions';
COMMENT ON COLUMN form_master.section_text IS 'Full descriptive text heading of the BRSR Section';
COMMENT ON COLUMN form_master.principle_text IS 'Full descriptive text heading of the Principle (Principles 1 through 9)';
COMMENT ON COLUMN form_master.sub_section_text IS 'Descriptive text heading of the Sub-Section within Section A or B';
COMMENT ON COLUMN form_master.indicator_text IS 'Descriptive indicator classification heading (Essential vs Leadership)';
COMMENT ON COLUMN form_master.question_text IS 'Exact full text of the question as presented in the filing form';
COMMENT ON COLUMN form_master.indicator_capmkt IS 'Raw XBRL tag name within the SEBI namespace (in-capmkt)';
COMMENT ON COLUMN form_master.context_reference IS 'Raw XBRL context reference attribute (e.g., DCYMain, ICYMain, D_StockExchangeAxis1)';
COMMENT ON COLUMN form_master.standard_indicator_capmkt IS 'Standardized, canonical XBRL tag name across filing years';
COMMENT ON COLUMN form_master.standard_context_reference IS 'Standardized, canonical context reference tag across filing years';
COMMENT ON COLUMN form_master.response_options IS 'Permitted enumeration options or dropdown selections for structured inputs';
COMMENT ON COLUMN form_master.input_text IS 'Guidance and contextual tooltip message provided to the filer';
COMMENT ON COLUMN form_master.formula IS 'Mathematical formula definition for calculated fields';
COMMENT ON COLUMN form_master.comments IS 'Supporting technical notes and domain guidance for the field';

-- 3. ENTITY_LIST COMMENTS
COMMENT ON TABLE entity_list IS 'Horizontal master record of corporate profiles, financial periods, and NIC industry classifications';
COMMENT ON COLUMN entity_list.entity_id IS 'Unique UUID identifier for the company filing record';
COMMENT ON COLUMN entity_list.section IS 'BRSR Section identifier (Default: A)';
COMMENT ON COLUMN entity_list.principle_number IS 'Principle number (Default: 0 for Section A)';
COMMENT ON COLUMN entity_list.sub_section IS 'Sub-section identifier (Default: 1 for General Disclosures)';
COMMENT ON COLUMN entity_list.friendly_group IS 'Grouping identifier (Default: entity_list)';
COMMENT ON COLUMN entity_list.stock_market IS 'Exchange source where filing was retrieved (BSE or NSE)';
COMMENT ON COLUMN entity_list.file_uuid IS 'Foreign key referencing file_list(file_id) to trace the source filing file';
COMMENT ON COLUMN entity_list.corporate_id_number IS 'Corporate Identity Number (CIN) issued by MCA';
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

-- 4. BRSR_FACTS COMMENTS
COMMENT ON TABLE brsr_facts IS 'Granular Entity-Attribute-Value (EAV) table containing all extracted quantitative and qualitative disclosures';
COMMENT ON COLUMN brsr_facts.brsr_fact_id IS 'Unique UUID primary key for each extracted disclosure fact';
COMMENT ON COLUMN brsr_facts.section IS 'BRSR Section (A, B, or C)';
COMMENT ON COLUMN brsr_facts.principle_number IS 'Principle Number (0 to 9; 0 for Sections A and B)';
COMMENT ON COLUMN brsr_facts.indicator_type IS 'Indicator classification - Essential, Leadership, or General';
COMMENT ON COLUMN brsr_facts.sub_section IS 'BRSR Sub-Section (1 to 7 for Section A; 0 for B and C)';
COMMENT ON COLUMN brsr_facts.client_id IS 'Corporate Identity Number (CIN) identifying the filer';
COMMENT ON COLUMN brsr_facts.file_uuid IS 'Foreign key referencing file_list(file_id) to link fact to source file and upload timestamp';
COMMENT ON COLUMN brsr_facts.client_uuid IS 'Foreign key referencing entity_list(entity_id) linking fact to company profile record';
COMMENT ON COLUMN brsr_facts.indicator_capmkt IS 'Raw SEBI XBRL tag as parsed from the XML document';
COMMENT ON COLUMN brsr_facts.standard_indicator_capmkt IS 'Standardized taxonomy concept tag name mapped via form_master';
COMMENT ON COLUMN brsr_facts.context_reference IS 'Raw context reference string from the XML fact element';
COMMENT ON COLUMN brsr_facts.standard_context_reference IS 'Standardized context reference string mapped via form_master';
COMMENT ON COLUMN brsr_facts.friendly_grouping IS 'Sub-table or analytical grouping (e.g., details_other_listed_entity, energy_consumption)';
COMMENT ON COLUMN brsr_facts.response_datatype IS 'Inferred data type: number or string';
COMMENT ON COLUMN brsr_facts.raw_value IS 'Original verbatim text content as submitted in the XML element';
COMMENT ON COLUMN brsr_facts.numeric_value IS 'Parsed numeric representation of raw_value (if convertible)';
COMMENT ON COLUMN brsr_facts.unit_reference IS 'Raw unit identifier from XML (e.g., INR, KL, Tonne, GJPerINR)';
COMMENT ON COLUMN brsr_facts.standard_unit_reference IS 'Human-readable raw unit representation resolved from xbrli:unit element';
COMMENT ON COLUMN brsr_facts.decimals IS 'Decimal precision attribute specified on the numeric XBRL fact';
COMMENT ON COLUMN brsr_facts.standard_raw_value IS 'Standardized text or normalized string representation';
COMMENT ON COLUMN brsr_facts.standard_numeric_value IS 'Canonical mathematically converted value (e.g., all energy converted to GJ, mass to Tonnes, GHG to GtCO2e)';
COMMENT ON COLUMN brsr_facts.created_at IS 'Timestamp when the disclosure fact was inserted into the database';

-- 5. UNIT_CONVERSIONS COMMENTS
COMMENT ON TABLE unit_conversions IS 'Mathematical conversion multipliers standardizing arbitrary engineering units into base SI metrics';
COMMENT ON COLUMN unit_conversions.conversion_id IS 'Unique UUID for the unit rule';
COMMENT ON COLUMN unit_conversions.metric_category IS 'Category of measurement (Energy, Volume, Mass, GHG, Concentration)';
COMMENT ON COLUMN unit_conversions.raw_unit IS 'Upper-case alphanumeric normalized unit label from XBRL (Unique key)';
COMMENT ON COLUMN unit_conversions.canonical_unit IS 'Target base unit (GJ, t, KL, GtCO2e)';
COMMENT ON COLUMN unit_conversions.conversion_factor IS 'Mathematical multiplier to obtain the canonical value';
COMMENT ON COLUMN unit_conversions.notes IS 'Supporting domain derivation notes';
COMMENT ON COLUMN unit_conversions.created_at IS 'Timestamp when the rule was registered';

-- 6. INDUSTRY_MASTER COMMENTS
COMMENT ON TABLE industry_master IS 'Master taxonomy mapping Indian NIC codes (from CIN chars 2-6) to MOSPI divisions and custom ESG benchmarking peer groups.';
COMMENT ON COLUMN industry_master.industry_id IS 'Unique UUID for the industry mapping record';
COMMENT ON COLUMN industry_master.nic_code IS 'Primary Key. 5-digit NIC code (from CIN chars 2-6, e.g. 62011) or 2-digit fallback code (e.g. 62).';
COMMENT ON COLUMN industry_master.division_code IS '2-digit MOSPI Division number (01 to 99), representing the broad economic activity.';
COMMENT ON COLUMN industry_master.division_name IS 'Official MOSPI title describing the 2-digit division.';
COMMENT ON COLUMN industry_master.section_code IS 'Top-level single-letter category (A to U) in the NIC/ISIC classification hierarchy.';
COMMENT ON COLUMN industry_master.section_name IS 'Official macro-level title corresponding to the section letter (e.g., Manufacturing, Information and Communication).';
COMMENT ON COLUMN industry_master.esg_sector IS 'Actionable ESG peer-group label used for BRSR analytics, emissions intensity comparisons, and AI prompts.';


ALTER SYSTEM SET io_method = 'worker'; 
ALTER SYSTEM SET io_combine_limit = 128; 
ALTER SYSTEM SET shared_buffers = '1GB'; 
ALTER SYSTEM SET work_mem = '64MB'; 
ALTER SYSTEM SET maintenance_work_mem = '256MB'; 
ALTER SYSTEM SET synchronous_commit = 'off'; 
ALTER SYSTEM SET random_page_cost = 1.1; 
ALTER SYSTEM SET effective_io_concurrency = 256; 
ALTER SYSTEM SET max_parallel_workers_per_gather = 4;