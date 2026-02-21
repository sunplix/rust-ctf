BEGIN;

ALTER TABLE site_settings
  ADD COLUMN IF NOT EXISTS time_display_mode VARCHAR(16) NOT NULL DEFAULT 'utc';

ALTER TABLE site_settings
  DROP CONSTRAINT IF EXISTS site_settings_time_display_mode_check;

ALTER TABLE site_settings
  ADD CONSTRAINT site_settings_time_display_mode_check
  CHECK (time_display_mode IN ('local', 'utc'));

ALTER TABLE contests
  ADD COLUMN IF NOT EXISTS registration_requires_approval BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE IF NOT EXISTS contest_registrations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  status VARCHAR(16) NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'approved', 'rejected')),
  requested_by UUID REFERENCES users(id) ON DELETE SET NULL,
  requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL,
  reviewed_at TIMESTAMPTZ,
  review_note TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (contest_id, team_id),
  CHECK (
    (status = 'pending' AND reviewed_at IS NULL)
    OR (status IN ('approved', 'rejected') AND reviewed_at IS NOT NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_contest_registrations_contest_status
  ON contest_registrations (contest_id, status, requested_at DESC);

CREATE INDEX IF NOT EXISTS idx_contest_registrations_team
  ON contest_registrations (team_id, contest_id, status);

DROP TRIGGER IF EXISTS trg_contest_registrations_touch_updated_at ON contest_registrations;
CREATE TRIGGER trg_contest_registrations_touch_updated_at
BEFORE UPDATE ON contest_registrations
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

ALTER TABLE challenges
  ADD COLUMN IF NOT EXISTS hints TEXT[] NOT NULL DEFAULT '{}'::text[];

COMMIT;
