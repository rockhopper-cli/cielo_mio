-- create table - Official BRSR Form Explorer Layout & Visibility Rules
CREATE TABLE form_layout (
    layout_id UUID PRIMARY KEY DEFAULT uuidv7(),
    question_key VARCHAR(64) NOT NULL UNIQUE,
    sort_order INTEGER NOT NULL UNIQUE,
    section VARCHAR(10) NOT NULL,
    principle_number SMALLINT NOT NULL DEFAULT 0,
    sub_section SMALLINT NOT NULL DEFAULT 0,
    indicator_type VARCHAR(20) NOT NULL DEFAULT '0',
    question_in_form SMALLINT NULL,
    question_number SMALLINT NOT NULL,
    followup_group VARCHAR(20) NULL,
    render_kind VARCHAR(20) NOT NULL,
    section_text VARCHAR(255) NULL,
    principle_text VARCHAR(255) NULL,
    sub_section_text VARCHAR(255) NULL,
    indicator_text VARCHAR(255) NULL,
    question_text TEXT NOT NULL,
    std_tag TEXT NULL,
    std_context TEXT NULL,
    response_datatype VARCHAR(50) NOT NULL,
    is_axis_increment BOOLEAN NOT NULL DEFAULT FALSE,
    append_to_field VARCHAR(20) NULL,
    friendly_grouping VARCHAR(255) NULL,
    visibility_parent_key VARCHAR(64) NULL,
    visibility_trigger VARCHAR(255) NULL,
    input_text TEXT NULL
);

-- index
CREATE INDEX idx_form_layout_nav_order
ON form_layout (section, principle_number, sub_section, sort_order);

CREATE INDEX idx_form_layout_std_lookup
ON form_layout (std_tag, std_context)
WHERE std_tag IS NOT NULL;

CREATE INDEX idx_form_layout_visibility_parent
ON form_layout (visibility_parent_key)
WHERE visibility_parent_key IS NOT NULL;

-- function - Sanitize mojibake, sentinel zeros, and whitespace on load/edit
CREATE OR REPLACE FUNCTION clean_brsr_text(val TEXT)
RETURNS TEXT AS $$
DECLARE
    res TEXT;
BEGIN
    IF val IS NULL THEN
        RETURN NULL;
    END IF;

    res := val;
    res := replace(res, U&'\00e2\20ac\2122', '''');         -- â€™ -> '
    res := replace(res, U&'\00e2\20ac\02dc', '''');         -- â€˜ -> '
    res := replace(res, U&'\00e2\20ac\0153', '"');          -- â€œ -> "
    res := replace(res, U&'\00e2\20ac\009d', '"');          -- â€<0x9d> -> "
    res := replace(res, U&'\00e2\20ac\201c', U&'\2013');    -- â€“ -> –
    res := replace(res, U&'\00e2\20ac\201d', U&'\2014');    -- â€” -> —
    res := replace(res, U&'\00e2\201a\00b9', U&'\20b9');    -- â‚¹ -> ₹
    res := replace(res, U&'\00c2\00a0', ' ');              -- Â<nbsp> -> space
    res := replace(res, U&'\00a0', ' ');                    -- <nbsp> -> space
    res := replace(res, U&'\00c2 ', ' ');                   -- "Â " -> space
    res := replace(res, U&'\00c2', '');                     -- stray Â -> ""
    res := regexp_replace(res, U&'\00e2\20ac' || '(?=[\s,;.)]|$)', '"', 'g');
    res := replace(res, '(In )', U&'(In \20b9)');
    res := regexp_replace(res, '[ \t]{2,}', ' ', 'g');

    RETURN NULLIF(TRIM(res), '');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION trg_clean_form_layout()
RETURNS TRIGGER AS $$
BEGIN
    NEW.question_key          := TRIM(NEW.question_key);
    NEW.section               := TRIM(NEW.section);
    NEW.indicator_type        := TRIM(NEW.indicator_type);
    NEW.followup_group        := NULLIF(NULLIF(TRIM(NEW.followup_group), '0'), '');
    NEW.render_kind           := TRIM(NEW.render_kind);
    NEW.section_text          := clean_brsr_text(NEW.section_text);
    NEW.principle_text        := NULLIF(clean_brsr_text(NEW.principle_text), 'NA');
    NEW.sub_section_text      := NULLIF(clean_brsr_text(NEW.sub_section_text), 'NA');
    NEW.indicator_text        := NULLIF(clean_brsr_text(NEW.indicator_text), 'NA');
    NEW.question_text         := COALESCE(clean_brsr_text(NEW.question_text), '');
    NEW.std_tag               := NULLIF(NULLIF(NULLIF(TRIM(NEW.std_tag), '0'), ''), 'FORMULA_IGNORE');
    NEW.std_context           := NULLIF(NULLIF(TRIM(NEW.std_context), '0'), '');
    NEW.response_datatype     := TRIM(NEW.response_datatype);
    NEW.append_to_field       := NULLIF(NULLIF(TRIM(NEW.append_to_field), '0'), '');
    NEW.friendly_grouping     := NULLIF(NULLIF(TRIM(NEW.friendly_grouping), '0'), '');
    NEW.visibility_parent_key := NULLIF(NULLIF(TRIM(NEW.visibility_parent_key), '0'), '');
    NEW.visibility_trigger    := NULLIF(NULLIF(TRIM(NEW.visibility_trigger), '0'), '');
    NEW.input_text            := clean_brsr_text(NEW.input_text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- trigger
CREATE OR REPLACE TRIGGER trg_form_layout_sanitize
BEFORE INSERT OR UPDATE ON form_layout
FOR EACH ROW
EXECUTE FUNCTION trg_clean_form_layout();

-- comments 26
COMMENT ON TABLE form_layout IS 'Canonical UI layout, question hierarchy, formatting datatypes, and conditional visibility rules for the BRSR Form Explorer';
COMMENT ON COLUMN form_layout.layout_id IS 'Unique UUID primary key for each form layout entry';
COMMENT ON COLUMN form_layout.question_key IS 'Globally unique canonical identifier for each active form question or detail line item';
COMMENT ON COLUMN form_layout.sort_order IS 'Strictly monotonic integer defining exact statutory display order across Sections A, B, and C';
COMMENT ON COLUMN form_layout.section IS 'BRSR Section - A (General), B (Management), or C (Principles)';
COMMENT ON COLUMN form_layout.principle_number IS 'NGRBC Principle Number (1 to 9 for Section C; 0 for Sections A and B)';
COMMENT ON COLUMN form_layout.sub_section IS 'BRSR Sub-Section (1 to 7 for Section A; 0 for Sections B and C)';
COMMENT ON COLUMN form_layout.indicator_type IS 'Indicator classification - e (Essential), l (Leadership), or 0 (General/NA)';
COMMENT ON COLUMN form_layout.question_in_form IS 'Displayed statutory question number on the official SEBI form (NULL for sub-headings and assurance gates)';
COMMENT ON COLUMN form_layout.question_number IS 'Internal sequential question grouping number used to cluster related detail rows into cards and tables';
COMMENT ON COLUMN form_layout.followup_group IS 'Sub-question branch identifier (e.g., a, b, c, i, ii) within a primary question';
COMMENT ON COLUMN form_layout.render_kind IS 'Explicit UI renderer hint: heading, scalar, table_cell, axis_col, or matrix_9p';
COMMENT ON COLUMN form_layout.section_text IS 'Full descriptive heading of the BRSR Section';
COMMENT ON COLUMN form_layout.principle_text IS 'Full descriptive heading of the Principle (Principles 1 through 9)';
COMMENT ON COLUMN form_layout.sub_section_text IS 'Descriptive heading of the Sub-Section within Section A or B';
COMMENT ON COLUMN form_layout.indicator_text IS 'Descriptive indicator classification heading (Essential Indicators vs Leadership Indicators)';
COMMENT ON COLUMN form_layout.question_text IS 'Cleaned verbatim prompt or line-item label displayed in the Form Explorer';
COMMENT ON COLUMN form_layout.std_tag IS 'Canonical standardized XBRL concept tag linking this row to brsr_facts (NULL for headings)';
COMMENT ON COLUMN form_layout.std_context IS 'Canonical standardized contextRef linking this row to brsr_facts (NULL for headings)';
COMMENT ON COLUMN form_layout.response_datatype IS 'Expected display datatype: number, string, date, boolean, email, phone, or hyperlink';
COMMENT ON COLUMN form_layout.is_axis_increment IS 'TRUE if this column repeats dynamically across Axis1..AxisN rows in a relational table';
COMMENT ON COLUMN form_layout.append_to_field IS 'Optional display suffix tag (e.g., _PY for Prior Year labels)';
COMMENT ON COLUMN form_layout.friendly_grouping IS 'Analytical functional grouping identifier used for table grouping and AI Copilot queries';
COMMENT ON COLUMN form_layout.visibility_parent_key IS 'question_key of the controlling gate question whose response shows or hides this row (NULL if always visible)';
COMMENT ON COLUMN form_layout.visibility_trigger IS 'Required controlling response expression (e.g., Yes, No, NA, All, <=10, ANYVALUE)';
COMMENT ON COLUMN form_layout.input_text IS 'Contextual guidance and explanatory note displayed beneath the question title';