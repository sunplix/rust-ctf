BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION touch_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$;

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username VARCHAR(32) NOT NULL CHECK (username ~ '^[A-Za-z0-9_-]{3,32}$'),
  email VARCHAR(128) NOT NULL,
  password_hash TEXT NOT NULL,
  role VARCHAR(16) NOT NULL DEFAULT 'player' CHECK (role IN ('player', 'admin', 'judge')),
  status VARCHAR(16) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uk_users_username_lower ON users ((LOWER(username)));
CREATE UNIQUE INDEX uk_users_email_lower ON users ((LOWER(email)));

CREATE TABLE teams (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL,
  captain_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  description TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uk_teams_name_lower ON teams ((LOWER(name)));

CREATE TABLE team_members (
  team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  member_role VARCHAR(16) NOT NULL DEFAULT 'member' CHECK (member_role IN ('captain', 'member')),
  joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (team_id, user_id),
  UNIQUE (user_id)
);

CREATE TABLE contests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title VARCHAR(160) NOT NULL,
  slug VARCHAR(120) NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  visibility VARCHAR(16) NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private')),
  status VARCHAR(16) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'running', 'ended', 'archived')),
  start_at TIMESTAMPTZ NOT NULL,
  end_at TIMESTAMPTZ NOT NULL,
  freeze_at TIMESTAMPTZ,
  created_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (end_at > start_at),
  CHECK (freeze_at IS NULL OR (freeze_at >= start_at AND freeze_at <= end_at))
);

CREATE UNIQUE INDEX uk_contests_slug_lower ON contests ((LOWER(slug)));

CREATE TABLE challenges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title VARCHAR(160) NOT NULL,
  slug VARCHAR(120) NOT NULL,
  category VARCHAR(32) NOT NULL,
  difficulty VARCHAR(16) NOT NULL DEFAULT 'normal' CHECK (difficulty IN ('easy', 'normal', 'hard', 'insane')),
  description TEXT NOT NULL DEFAULT '',
  static_score INTEGER NOT NULL DEFAULT 100 CHECK (static_score > 0),
  min_score INTEGER NOT NULL DEFAULT 50 CHECK (min_score >= 0),
  max_score INTEGER NOT NULL DEFAULT 500 CHECK (max_score >= min_score),
  challenge_type VARCHAR(16) NOT NULL DEFAULT 'static' CHECK (challenge_type IN ('static', 'dynamic', 'internal')),
  flag_mode VARCHAR(16) NOT NULL DEFAULT 'static' CHECK (flag_mode IN ('static', 'dynamic', 'script')),
  flag_hash TEXT NOT NULL DEFAULT '',
  compose_template TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_visible BOOLEAN NOT NULL DEFAULT FALSE,
  created_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uk_challenges_slug_lower ON challenges ((LOWER(slug)));
CREATE INDEX idx_challenges_category ON challenges (category);
CREATE INDEX idx_challenges_visible ON challenges (is_visible);

CREATE TABLE contest_challenges (
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  release_at TIMESTAMPTZ,
  PRIMARY KEY (contest_id, challenge_id)
);

CREATE INDEX idx_contest_challenges_sort ON contest_challenges (contest_id, sort_order, challenge_id);

CREATE TABLE submissions (
  id BIGSERIAL PRIMARY KEY,
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  submitted_flag TEXT NOT NULL,
  verdict VARCHAR(16) NOT NULL CHECK (verdict IN ('accepted', 'wrong', 'invalid', 'rate_limited', 'error')),
  score_awarded INTEGER NOT NULL DEFAULT 0,
  judger_message TEXT NOT NULL DEFAULT '',
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  judged_at TIMESTAMPTZ
);

CREATE INDEX idx_submissions_contest_team_time ON submissions (contest_id, team_id, submitted_at DESC);
CREATE INDEX idx_submissions_contest_challenge_verdict ON submissions (contest_id, challenge_id, verdict);
CREATE INDEX idx_submissions_team_challenge_accepted ON submissions (team_id, challenge_id) WHERE verdict = 'accepted';

CREATE TABLE instances (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  subnet CIDR NOT NULL,
  compose_project_name VARCHAR(96) NOT NULL,
  status VARCHAR(16) NOT NULL DEFAULT 'creating' CHECK (status IN ('creating', 'running', 'stopped', 'destroyed', 'expired', 'failed')),
  entrypoint_url TEXT NOT NULL DEFAULT '',
  cpu_limit NUMERIC(5,2),
  memory_limit_mb INTEGER,
  started_at TIMESTAMPTZ,
  expires_at TIMESTAMPTZ,
  destroyed_at TIMESTAMPTZ,
  last_heartbeat_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (contest_id, challenge_id, team_id),
  UNIQUE (subnet),
  UNIQUE (compose_project_name)
);

CREATE INDEX idx_instances_status_expire ON instances (status, expires_at);
CREATE INDEX idx_instances_team ON instances (team_id, created_at DESC);

CREATE TRIGGER trg_users_touch_updated_at
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

CREATE TRIGGER trg_teams_touch_updated_at
BEFORE UPDATE ON teams
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

CREATE TRIGGER trg_contests_touch_updated_at
BEFORE UPDATE ON contests
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

CREATE TRIGGER trg_challenges_touch_updated_at
BEFORE UPDATE ON challenges
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

CREATE TRIGGER trg_instances_touch_updated_at
BEFORE UPDATE ON instances
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
