BEGIN;

CREATE TABLE contest_scoreboard_channels (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  contest_id UUID NOT NULL REFERENCES contests(id) ON DELETE CASCADE,
  name VARCHAR(80) NOT NULL,
  invite_code VARCHAR(48) NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (id, contest_id)
);

CREATE UNIQUE INDEX uk_contest_scoreboard_channels_name_lower
  ON contest_scoreboard_channels (contest_id, LOWER(name));

CREATE UNIQUE INDEX uk_contest_scoreboard_channels_invite_code_lower
  ON contest_scoreboard_channels (contest_id, LOWER(invite_code));

CREATE INDEX idx_contest_scoreboard_channels_contest_active
  ON contest_scoreboard_channels (contest_id, is_active, created_at DESC);

CREATE TABLE contest_scoreboard_channel_members (
  contest_id UUID NOT NULL,
  channel_id UUID NOT NULL,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  joined_by_invite_code VARCHAR(48) NOT NULL DEFAULT '',
  joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (channel_id, user_id),
  FOREIGN KEY (channel_id, contest_id)
    REFERENCES contest_scoreboard_channels(id, contest_id)
    ON DELETE CASCADE,
  UNIQUE (contest_id, user_id)
);

CREATE INDEX idx_contest_scoreboard_channel_members_contest_channel
  ON contest_scoreboard_channel_members (contest_id, channel_id, joined_at DESC);

CREATE INDEX idx_contest_scoreboard_channel_members_user
  ON contest_scoreboard_channel_members (user_id, contest_id);

CREATE TRIGGER trg_contest_scoreboard_channels_touch_updated_at
BEFORE UPDATE ON contest_scoreboard_channels
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
