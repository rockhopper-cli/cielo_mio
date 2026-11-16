-- create table - XBRL Ingestion & Canonical Tag Mapping Rules
CREATE TABLE xbrl_tag_rules (
    rule_id UUID PRIMARY KEY DEFAULT uuidv7(),
    raw_tag TEXT NOT NULL,
    raw_context TEXT NOT NULL,
    std_tag TEXT NOT NULL,
    std_context TEXT NOT NULL,
    is_axis_increment BOOLEAN NOT NULL DEFAULT FALSE,
    section VARCHAR(10) NOT NULL,
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 0,
    indicator_type VARCHAR(20) NOT NULL DEFAULT '0',
    friendly_grouping VARCHAR(255) NULL,
    is_legacy BOOLEAN NOT NULL DEFAULT FALSE,

    CONSTRAINT uq_xbrl_tag_rules_raw_pair UNIQUE (raw_tag, raw_context)
);

-- index
CREATE INDEX idx_xbrl_tag_rules_std
ON xbrl_tag_rules (std_tag, std_context);

CREATE INDEX idx_xbrl_tag_rules_grouping
ON xbrl_tag_rules (friendly_grouping)
WHERE friendly_grouping IS NOT NULL;

CREATE INDEX idx_xbrl_tag_rules_axis
ON xbrl_tag_rules (raw_tag)
WHERE is_axis_increment = TRUE;

-- comments 13
COMMENT ON TABLE xbrl_tag_rules IS 'ETL dictionary mapping raw/legacy SEBI XBRL tags and contexts to canonical tags, axes, sections, and analytical groupings';
COMMENT ON COLUMN xbrl_tag_rules.rule_id IS 'Unique UUID primary key for each XBRL mapping rule';
COMMENT ON COLUMN xbrl_tag_rules.raw_tag IS 'Raw XBRL tag name within the SEBI in-capmkt namespace as submitted in XML filings';
COMMENT ON COLUMN xbrl_tag_rules.raw_context IS 'Raw XBRL contextRef attribute (e.g., DCYMain, ICYMain, D_StockExchangeAxis1)';
COMMENT ON COLUMN xbrl_tag_rules.std_tag IS 'Canonical standardized XBRL concept tag across filing years and taxonomy revisions';
COMMENT ON COLUMN xbrl_tag_rules.std_context IS 'Canonical standardized contextRef identifier across filing years';
COMMENT ON COLUMN xbrl_tag_rules.is_axis_increment IS 'TRUE signals dynamic numeric suffix cloning (e.g., Axis1, Axis2...) during XBRL enrichment';
COMMENT ON COLUMN xbrl_tag_rules.section IS 'BRSR Section - A (General), B (Management), or C (Principles)';
COMMENT ON COLUMN xbrl_tag_rules.principle_number IS 'NGRBC Principle Number (1 to 9 for Section C; 0 for Sections A and B)';
COMMENT ON COLUMN xbrl_tag_rules.sub_section IS 'BRSR Sub-Section (1 to 7 for Section A; 0 for Sections B and C)';
COMMENT ON COLUMN xbrl_tag_rules.indicator_type IS 'Indicator classification - e (Essential), l (Leadership), or 0 (General/NA)';
COMMENT ON COLUMN xbrl_tag_rules.friendly_grouping IS 'Analytical functional grouping identifier (e.g., entity_list, details_ghg_emission) used for exports and AI tools';
COMMENT ON COLUMN xbrl_tag_rules.is_legacy IS 'TRUE flags deprecated/typo taxonomy tag mappings retained for historical filing ingestion';