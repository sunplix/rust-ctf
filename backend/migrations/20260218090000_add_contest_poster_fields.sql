BEGIN;

ALTER TABLE contests
  ADD COLUMN poster_storage_path TEXT,
  ADD COLUMN poster_content_type VARCHAR(128);

COMMIT;
