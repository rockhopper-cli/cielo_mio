-- create table - Indian Postal Code Master & Auto-Resolution Trigger
CREATE TABLE geo_pincode (
    geo_id UUID PRIMARY KEY DEFAULT uuidv7(),
    country_code VARCHAR(2) NOT NULL DEFAULT 'IN',
    postal_code VARCHAR(20) NOT NULL,
    place_name VARCHAR(180) NOT NULL,
    admin_name1 VARCHAR(100) NULL,
    admin_code1 VARCHAR(20) NULL,
    admin_name2 VARCHAR(100) NULL,
    admin_code2 VARCHAR(20) NULL,
    admin_name3 VARCHAR(100) NULL,
    admin_code3 VARCHAR(20) NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    accuracy SMALLINT NULL DEFAULT 4
);

-- index
CREATE INDEX idx_geo_pincode_lookup ON geo_pincode (postal_code);
CREATE INDEX idx_geo_pincode_coords ON geo_pincode (latitude, longitude);

-- comments 14
COMMENT ON TABLE geo_pincode IS 'Indian Postal Index Number (PIN) master reference table for offline entity coordinate and state/city resolution';
COMMENT ON COLUMN geo_pincode.geo_id IS 'Unique UUID primary key for each postal code locality entry';
COMMENT ON COLUMN geo_pincode.country_code IS 'ISO 3166-1 alpha-2 country code (Default: IN for India)';
COMMENT ON COLUMN geo_pincode.postal_code IS '6-digit Indian PIN code (or international postal code) used as the geocoding lookup key';
COMMENT ON COLUMN geo_pincode.place_name IS 'Locality, post office, town, or village name associated with the postal code';
COMMENT ON COLUMN geo_pincode.admin_name1 IS '1st-order administrative subdivision name (State or Union Territory)';
COMMENT ON COLUMN geo_pincode.admin_code1 IS '1st-order administrative subdivision code (State / UT identifier)';
COMMENT ON COLUMN geo_pincode.admin_name2 IS '2nd-order administrative subdivision name (District)';
COMMENT ON COLUMN geo_pincode.admin_code2 IS '2nd-order administrative subdivision code (District identifier)';
COMMENT ON COLUMN geo_pincode.admin_name3 IS '3rd-order administrative subdivision name (Taluk, Tehsil, or Sub-district)';
COMMENT ON COLUMN geo_pincode.admin_code3 IS '3rd-order administrative subdivision code (Taluk / Tehsil identifier)';
COMMENT ON COLUMN geo_pincode.latitude IS 'Estimated WGS84 latitude coordinate in decimal degrees';
COMMENT ON COLUMN geo_pincode.longitude IS 'Estimated WGS84 longitude coordinate in decimal degrees';
COMMENT ON COLUMN geo_pincode.accuracy IS 'Coordinate precision rating from 1 (estimated) to 6 (centroid); Default: 4';

-- function
CREATE OR REPLACE FUNCTION trg_resolve_entity_coordinates()
RETURNS TRIGGER AS $$
DECLARE
    raw_addr TEXT;
    pin_match TEXT[];
    clean_pin VARCHAR(6);
    geo RECORD;
BEGIN
    raw_addr := CONCAT_WS(' ',
        NULLIF(TRIM(NEW.address_registered_office), ''),
        NULLIF(TRIM(NEW.address_corporate_office), '')
    );

    IF raw_addr <> '' THEN
        NEW.geocode_status := 'NO_PINCODE';
        -- Iterate through all 6-digit PIN candidates (reversed preference via array check)
        FOR pin_match IN
            SELECT regexp_matches(raw_addr, '\y([1-9][0-9]{2}\s?[0-9]{3})\y', 'g')
        LOOP
            clean_pin := replace(pin_match[1], ' ', '');

            SELECT
                ROUND(AVG(latitude)::numeric, 4)::double precision AS latitude,
                ROUND(AVG(longitude)::numeric, 4)::double precision AS longitude,
                MODE() WITHIN GROUP (ORDER BY admin_name1) AS state_name,
                MODE() WITHIN GROUP (ORDER BY admin_name2) AS district_name
            INTO geo
            FROM geo_pincode
            WHERE postal_code = clean_pin;

            IF geo.latitude IS NOT NULL THEN
                NEW.latitude := geo.latitude;
                NEW.longitude := geo.longitude;
                NEW.resolved_state := COALESCE(geo.state_name, 'Unknown State');
                NEW.resolved_city := COALESCE(geo.district_name, 'Unknown City');
                NEW.geocode_status := 'RESOLVED';
                RETURN NEW;
            ELSE
                NEW.geocode_status := 'NOT_FOUND';
            END IF;
        END LOOP;
    ELSE
        NEW.geocode_status := 'NO_ADDRESS';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

