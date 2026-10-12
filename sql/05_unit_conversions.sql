-- ============================================================================
-- 05_unit_conversions.sql
-- Normalization Multipliers & Canonical Conversion Seeds
-- ============================================================================

CREATE TABLE unit_conversions (
    conversion_id UUID PRIMARY KEY DEFAULT uuidv7(),
    metric_category VARCHAR(50) NOT NULL,
    raw_unit VARCHAR(50) NOT NULL UNIQUE,
    canonical_unit VARCHAR(50) NOT NULL,
    conversion_factor NUMERIC NOT NULL,         -- Fix F: Exact base-10 NUMERIC avoids binary float rounding
    notes TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- Indexes

CREATE INDEX idx_unit_raw_lookup ON unit_conversions (raw_unit);

-- Seed Multipliers

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
    ('Mass', 'G', 't', 0.000001, '1 Gram = 0.000001 Tonne'),
    ('Mass', 'GRAM', 't', 0.000001, 'Gram full word'),
    ('Mass', 'MG', 't', 0.000000001, '1 Milligram = 1e-9 Tonne'),
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
    ('GHG', 'GTCO2E', 't', 1000000000, '1 Gigatonne CO2 equivalent = 1e9 Tonnes')
ON CONFLICT (raw_unit) DO NOTHING;

-- ----------------------------------------------------------------------------
-- 5. UNIT_CONVERSIONS (8 Comments)
-- ----------------------------------------------------------------------------
COMMENT ON TABLE unit_conversions IS 'Mathematical conversion multipliers standardizing arbitrary engineering units into base SI metrics';
COMMENT ON COLUMN unit_conversions.conversion_id IS 'Unique UUID for the unit rule';
COMMENT ON COLUMN unit_conversions.metric_category IS 'Category of measurement (Energy, Volume, Mass, GHG, Concentration)';
COMMENT ON COLUMN unit_conversions.raw_unit IS 'Upper-case alphanumeric normalized unit label from XBRL (Unique key)';
COMMENT ON COLUMN unit_conversions.canonical_unit IS 'Target base unit (GJ, t, KL, GtCO2e)';
COMMENT ON COLUMN unit_conversions.conversion_factor IS 'Exact base-10 mathematical multiplier to obtain the canonical value';
COMMENT ON COLUMN unit_conversions.notes IS 'Supporting domain derivation notes';
COMMENT ON COLUMN unit_conversions.created_at IS 'Timestamp when the rule was registered';
