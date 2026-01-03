CREATE TABLE job_state (
    job_id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    kind TEXT NOT NULL
);
