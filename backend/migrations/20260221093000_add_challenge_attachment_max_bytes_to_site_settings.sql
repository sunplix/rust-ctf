BEGIN;

ALTER TABLE site_settings
ADD COLUMN IF NOT EXISTS challenge_attachment_max_bytes BIGINT NOT NULL DEFAULT 20971520;

ALTER TABLE site_settings
DROP CONSTRAINT IF EXISTS site_settings_challenge_attachment_max_bytes_check;

ALTER TABLE site_settings
ADD CONSTRAINT site_settings_challenge_attachment_max_bytes_check
CHECK (challenge_attachment_max_bytes BETWEEN 1048576 AND 268435456);

COMMIT;
