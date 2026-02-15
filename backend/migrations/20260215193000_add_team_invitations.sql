BEGIN;

CREATE TABLE team_invitations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  inviter_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  invitee_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  status VARCHAR(16) NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'accepted', 'rejected', 'canceled', 'expired')),
  message TEXT NOT NULL DEFAULT '',
  responded_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_team_invitations_invitee_status
  ON team_invitations (invitee_user_id, status, created_at DESC);

CREATE INDEX idx_team_invitations_team_status
  ON team_invitations (team_id, status, created_at DESC);

CREATE UNIQUE INDEX uk_team_invitations_pending
  ON team_invitations (team_id, invitee_user_id)
  WHERE status = 'pending';

CREATE TRIGGER trg_team_invitations_touch_updated_at
BEFORE UPDATE ON team_invitations
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
