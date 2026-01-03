-- Alter job_state table to use enum types
ALTER TABLE job_state
    ALTER COLUMN status TYPE job_status USING status::job_status,
    ALTER COLUMN kind TYPE job_kind USING kind::job_kind;

-- Alter llms_txt table: split result into status and data
ALTER TABLE llms_txt
    RENAME COLUMN result TO result_data;

ALTER TABLE llms_txt
    ADD COLUMN result_status result_status NOT NULL DEFAULT 'ok';

-- Remove the default after adding the column
ALTER TABLE llms_txt
    ALTER COLUMN result_status DROP DEFAULT;
