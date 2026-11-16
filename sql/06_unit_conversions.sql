-- create table - Normalization Multipliers & Canonical Conversion Seeds
CREATE TABLE unit_conversions (
    conversion_id UUID PRIMARY KEY DEFAULT uuidv7(),
    metric_category VARCHAR(50) NOT NULL,
    raw_unit VARCHAR(50) NOT NULL UNIQUE,
    canonical_unit VARCHAR(50) NOT NULL,
    conversion_factor NUMERIC NOT NULL,
    notes TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- comments 8
COMMENT ON TABLE unit_conversions IS 'Mathematical conversion multipliers standardizing arbitrary engineering units into base SI metrics';
COMMENT ON COLUMN unit_conversions.conversion_id IS 'Unique UUID for the unit rule';
COMMENT ON COLUMN unit_conversions.metric_category IS 'Category of measurement (Energy, Volume, Mass, GHG, Concentration)';
COMMENT ON COLUMN unit_conversions.raw_unit IS 'Upper-case alphanumeric normalized unit label from XBRL (Unique key)';
COMMENT ON COLUMN unit_conversions.canonical_unit IS 'Target base unit (GJ, t, KL, GtCO2e)';
COMMENT ON COLUMN unit_conversions.conversion_factor IS 'Exact base-10 mathematical multiplier to obtain the canonical value';
COMMENT ON COLUMN unit_conversions.notes IS 'Supporting domain derivation notes';
COMMENT ON COLUMN unit_conversions.created_at IS 'Timestamp when the rule was registered';