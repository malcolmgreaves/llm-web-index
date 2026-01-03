-- Create enum types
CREATE TYPE job_status AS ENUM (
    'queued',    -- A newly created job
    'started',   -- Job manager started job
    'running',   -- Worker received job
    'success',   -- New or updated llms.txt file made and added to database
    'failure'    -- Worker failed
);

CREATE TYPE job_kind AS ENUM (
    'new',       -- New llms.txt fetch
    'update'     -- Update existing llms.txt
);

CREATE TYPE result_status AS ENUM (
    'ok',        -- Successfully fetched llms.txt
    'error'      -- Failed to fetch llms.txt
);

-- Create job_state table
CREATE TABLE job_state (
    job_id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    status job_status NOT NULL,
    kind job_kind NOT NULL
);

-- Create llms_txt table
CREATE TABLE llms_txt (
    job_id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    result_data TEXT NOT NULL,
    result_status result_status NOT NULL
);

-- Create GIN index for full-text search on url column
CREATE INDEX llms_txt_url_fts_idx ON llms_txt USING GIN (to_tsvector('english', url));
