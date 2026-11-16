-- create table - Download & Ingestion State Machine Table
CREATE TABLE file_list (
    file_id UUID PRIMARY KEY DEFAULT uuidv7(),
    file_name VARCHAR(255) NOT NULL UNIQUE,
    stock_market VARCHAR(50) NOT NULL,
    symbol VARCHAR(50) NULL,
    scrip_code VARCHAR(20) NULL,
    upload_date DATE NOT NULL,
    upload_time TIME WITHOUT TIME ZONE NOT NULL,
    file_type VARCHAR(50) NOT NULL,
    mapped BOOLEAN NOT NULL DEFAULT FALSE,
    unmapped_reason TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- index
CREATE INDEX idx_file_list_lookup ON file_list (file_name, mapped);
CREATE INDEX idx_file_list_market_date ON file_list (stock_market, upload_date DESC);
CREATE INDEX idx_file_list_pending_xml ON file_list (upload_date DESC, file_name ASC)
WHERE file_type = 'XML' AND mapped = FALSE AND unmapped_reason IS NULL;

-- comments 12
COMMENT ON TABLE file_list IS 'Tracks all downloaded BRSR XML/PDF files, exchange tickers, hash states, and ingestion status';
COMMENT ON COLUMN file_list.file_id IS 'Unique UUID primary key for each downloaded file';
COMMENT ON COLUMN file_list.file_name IS 'Downloaded file name (pdf or xml)';
COMMENT ON COLUMN file_list.stock_market IS 'Stock exchange source - BSE or NSE';
COMMENT ON COLUMN file_list.symbol IS 'Authoritative NSE ticker symbol captured from exchange download feed';
COMMENT ON COLUMN file_list.scrip_code IS 'Authoritative BSE scrip code captured from exchange download feed';
COMMENT ON COLUMN file_list.upload_date IS 'Upload date derived from the file name';
COMMENT ON COLUMN file_list.upload_time IS 'Upload time derived from the file name';
COMMENT ON COLUMN file_list.file_type IS 'File format - PDF or XML';
COMMENT ON COLUMN file_list.mapped IS 'Whether data was successfully parsed, mapped, and uploaded into entity_list and brsr_facts';
COMMENT ON COLUMN file_list.unmapped_reason IS 'Diagnostic reason if data failed to map (e.g., New Questions, Corrupt File, Download Timeout, Blank File)';
COMMENT ON COLUMN file_list.created_at IS 'Timestamp when the file record was registered in the database';