-- Revert llms_txt changes
ALTER TABLE llms_txt
    DROP COLUMN result_status;

ALTER TABLE llms_txt
    RENAME COLUMN result_data TO result;

-- Revert job_state changes
ALTER TABLE job_state
    ALTER COLUMN status TYPE TEXT,
    ALTER COLUMN kind TYPE TEXT;
