-- Remove the index and column
DROP INDEX IF EXISTS llms_txt_html_checksum_idx;
ALTER TABLE llms_txt DROP COLUMN html_checksum;
