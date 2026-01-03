CREATE TABLE llms_txt (
    job_id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    result TEXT NOT NULL
);

-- Create GIN index for full-text search on url column
CREATE INDEX llms_txt_url_fts_idx ON llms_txt USING GIN (to_tsvector('english', url));
