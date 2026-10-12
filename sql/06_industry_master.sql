-- ============================================================================
-- 06_industry_master.sql
-- NIC Macro Divisions & 5-Digit Specific Seeds
-- ============================================================================

CREATE TABLE industry_master (
    industry_id UUID PRIMARY KEY DEFAULT uuidv7(),
    nic_code VARCHAR(10) NOT NULL UNIQUE,      -- Fix G: Enforced unique taxonomy key
    division_code VARCHAR(5) NOT NULL,
    division_name VARCHAR(255) NOT NULL,
    section_code VARCHAR(2) NOT NULL,
    section_name VARCHAR(255) NOT NULL,
    esg_sector VARCHAR(100) NOT NULL
);

-- Indexes

CREATE INDEX idx_industry_division ON industry_master(division_code);

-- Seed Industry

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

-- ----------------------------------------------------------------------------
-- 6. INDUSTRY_MASTER (8 Comments)
-- ----------------------------------------------------------------------------
COMMENT ON TABLE industry_master IS 'Master taxonomy mapping Indian NIC codes (from CIN chars 2-6) to MOSPI divisions and custom ESG benchmarking peer groups.';
COMMENT ON COLUMN industry_master.industry_id IS 'Unique UUID for the industry mapping record';
COMMENT ON COLUMN industry_master.nic_code IS 'Unique Key. 5-digit NIC code (from CIN chars 2-6, e.g. 62011) or 2-digit fallback code (e.g. 62).';
COMMENT ON COLUMN industry_master.division_code IS '2-digit MOSPI Division number (01 to 99), representing the broad economic activity.';
COMMENT ON COLUMN industry_master.division_name IS 'Official MOSPI title describing the 2-digit division.';
COMMENT ON COLUMN industry_master.section_code IS 'Top-level single-letter category (A to U) in the NIC/ISIC classification hierarchy.';
COMMENT ON COLUMN industry_master.section_name IS 'Official macro-level title corresponding to the section letter (e.g., Manufacturing, Information and Communication).';
COMMENT ON COLUMN industry_master.esg_sector IS 'Actionable ESG peer-group label used for BRSR analytics, emissions intensity comparisons, and AI prompts.';