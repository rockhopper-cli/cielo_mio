-- ============================================================================
-- 02_form_master.sql
-- Master Taxonomy Dictionary & Dynamic Context Rules
-- ============================================================================

CREATE TABLE form_master (
    form_id UUID PRIMARY KEY DEFAULT uuidv7(),
    question_key VARCHAR(50) NOT NULL,
    section VARCHAR(10) NOT NULL,
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 0,
    indicator_type VARCHAR(20) NOT NULL,
    question_in_form SMALLINT NULL,
    question_number SMALLINT NOT NULL,
    followup_question_first VARCHAR(20) NOT NULL,
    followup_question_second VARCHAR(20) NULL,
    question_order SMALLINT NOT NULL,
    question_type VARCHAR(20) NOT NULL,
    response_datatype VARCHAR(50) NOT NULL,
    response_required BOOLEAN NOT NULL DEFAULT FALSE,
    calculation BOOLEAN NOT NULL DEFAULT FALSE,
    contextref_increment BOOLEAN NOT NULL DEFAULT FALSE,
    visibility_dependence VARCHAR(50) NOT NULL DEFAULT '0',
    visibility_trigger VARCHAR(255) NULL,
    total_field BOOLEAN NOT NULL DEFAULT FALSE,
    friendly_grouping VARCHAR(255) NULL,
    field_flag SMALLINT NULL,
    anchor_reference SMALLINT NULL,
    append_to_field VARCHAR(20) NULL,
    legacy BOOLEAN NOT NULL DEFAULT FALSE,
    section_text VARCHAR(255) NULL,
    principle_text VARCHAR(255) NULL,
    sub_section_text VARCHAR(255) NULL,
    indicator_text VARCHAR(255) NULL,
    question_text TEXT NOT NULL,
    indicator_capmkt TEXT NULL,
    context_reference TEXT NULL,
    standard_indicator_capmkt TEXT NULL,
    standard_context_reference TEXT NULL,
    response_options TEXT NULL,
    input_text TEXT NULL,
    formula TEXT NULL,
    comments TEXT NULL
);

-- Indexes

CREATE INDEX idx_form_master_join_keys
ON form_master (indicator_capmkt, context_reference);

CREATE INDEX idx_form_master_standard_keys
ON form_master (standard_indicator_capmkt, standard_context_reference);

CREATE INDEX idx_form_master_grouping
ON form_master (friendly_grouping);

-- ----------------------------------------------------------------------------
-- 2. FORM_MASTER (38 Comments)
-- ----------------------------------------------------------------------------
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
