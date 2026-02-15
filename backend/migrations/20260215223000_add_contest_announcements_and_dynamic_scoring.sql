BEGIN;

ALTER TABLE contests
  ADD COLUMN scoring_mode VARCHAR(16) NOT NULL DEFAULT 'static'
    CHECK (scoring_mode IN ('static', 'dynamic')),
  ADD COLUMN dynamic_decay INTEGER NOT NULL DEFAULT 20
    CHECK (dynamic_decay >= 1 AND dynamic_decay <= 100000);

CREATE INDEX idx_contests_scoring_mode ON contests (scoring_mode);

CREATE TABLE contest_announcements (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  title VARCHAR(160) NOT NULL,
  content TEXT NOT NULL,
  is_published BOOLEAN NOT NULL DEFAULT FALSE,
  is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
  published_at TIMESTAMPTZ,
  created_by UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK ((is_published = FALSE) OR (published_at IS NOT NULL))
);

CREATE INDEX idx_contest_announcements_contest_created
  ON contest_announcements (contest_id, created_at DESC);

CREATE INDEX idx_contest_announcements_published
  ON contest_announcements (contest_id, is_published, published_at DESC);

CREATE TRIGGER trg_contest_announcements_touch_updated_at
BEFORE UPDATE ON contest_announcements
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
