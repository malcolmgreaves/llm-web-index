-- Add html_checksum column
ALTER TABLE llms_txt
ADD COLUMN html_checksum VARCHAR(32);

-- Backfill checksums from existing HTML (RAW, not normalized)
-- NOTE: Existing records contain raw HTML, so we hash that.
-- Going forward, checksums will be computed from NORMALIZED HTML.
-- This mismatch is intentional - first cron run will trigger updates
-- for all existing URLs, which will recompute with normalized checksums.
-- This "self-healing" approach avoids complex Rust migration scripts.
UPDATE llms_txt
SET html_checksum = md5(html)
WHERE html_checksum IS NULL AND html IS NOT NULL;

-- Make column non-null after backfill
ALTER TABLE llms_txt
ALTER COLUMN html_checksum SET NOT NULL;

-- Create index for efficient lookups
CREATE INDEX llms_txt_html_checksum_idx ON llms_txt (html_checksum);

-- Add documentation
COMMENT ON COLUMN llms_txt.html_checksum IS 'MD5 checksum of normalized HTML for change detection';
