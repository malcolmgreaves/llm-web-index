-- Create enum type for job_state.status
CREATE TYPE job_status AS ENUM (
    'queued',    -- A newly created job
    'started',   -- Job manager started job
    'running',   -- Worker received job
    'success',   -- New or updated llms.txt file made and added to database
    'failure'    -- Worker failed
);

-- Create enum type for job_state.kind
CREATE TYPE job_kind AS ENUM (
    'new',
    'update'
);

-- Create enum type for llms_txt.result_status
CREATE TYPE result_status AS ENUM (
    'ok',
    'error'
);
