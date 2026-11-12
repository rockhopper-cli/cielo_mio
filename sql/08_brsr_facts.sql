-- create table - Granular XBRL Facts EAV Repository
CREATE TABLE brsr_facts (
    brsr_fact_id UUID PRIMARY KEY DEFAULT uuidv7(),
    section VARCHAR(10) NOT NULL,
    principle_number SMALLINT NOT NULL DEFAULT 0,
    indicator_type VARCHAR(20) NOT NULL,
    sub_section SMALLINT NOT NULL DEFAULT 0,
    client_id VARCHAR(100) NOT NULL,
    file_uuid UUID NOT NULL REFERENCES file_list(file_id) ON DELETE CASCADE,
    client_uuid UUID NOT NULL REFERENCES entity_list(entity_id) ON DELETE CASCADE,
    indicator_capmkt TEXT NOT NULL,
    standard_indicator_capmkt TEXT NOT NULL,
    context_reference TEXT NOT NULL,
    standard_context_reference TEXT NOT NULL,
    friendly_grouping VARCHAR(255) NULL,
    response_datatype VARCHAR(50) NOT NULL,
    raw_value TEXT NULL,
    numeric_value NUMERIC(20, 4) NULL,
    unit_reference VARCHAR(50) NULL,
    standard_unit_reference VARCHAR(50) NULL,
    decimals SMALLINT NULL,
    standard_raw_value TEXT NULL,
    standard_numeric_value NUMERIC NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- index
CREATE INDEX idx_facts_client_uuid ON brsr_facts (client_uuid);
CREATE INDEX idx_facts_file_uuid ON brsr_facts (file_uuid);
CREATE INDEX idx_facts_standard_concept ON brsr_facts (standard_indicator_capmkt, standard_context_reference);
CREATE INDEX idx_facts_section_principle ON brsr_facts (section, principle_number);
CREATE INDEX idx_facts_skip_scan ON brsr_facts (friendly_grouping, client_uuid, standard_indicator_capmkt);
CREATE INDEX idx_facts_trgm_indicator ON brsr_facts USING gin (standard_indicator_capmkt gin_trgm_ops);
CREATE INDEX idx_facts_trgm_grouping ON brsr_facts USING gin (friendly_grouping gin_trgm_ops)
WHERE friendly_grouping IS NOT NULL;

-- comments 23
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
COMMENT ON COLUMN brsr_facts.standard_numeric_value IS 'Canonical mathematically converted value (arbitrary precision NUMERIC)';
COMMENT ON COLUMN brsr_facts.created_at IS 'Timestamp when the disclosure fact was inserted into the database';
