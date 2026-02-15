BEGIN;

CREATE TABLE runtime_alerts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  alert_type VARCHAR(64) NOT NULL,
  severity VARCHAR(16) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
  status VARCHAR(16) NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'acknowledged', 'resolved')),
  source_type VARCHAR(32) NOT NULL,
  source_id UUID,
  fingerprint VARCHAR(160) NOT NULL,
  title VARCHAR(255) NOT NULL,
  message TEXT NOT NULL,
  detail JSONB NOT NULL DEFAULT '{}'::jsonb,
  first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  acknowledged_at TIMESTAMPTZ,
  acknowledged_by UUID REFERENCES users(id) ON DELETE SET NULL,
  resolved_at TIMESTAMPTZ,
  resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_runtime_alerts_status_seen
  ON runtime_alerts (status, last_seen_at DESC);

CREATE INDEX idx_runtime_alerts_type_status_seen
  ON runtime_alerts (alert_type, status, last_seen_at DESC);

CREATE UNIQUE INDEX uk_runtime_alerts_active_fingerprint
  ON runtime_alerts (fingerprint)
  WHERE status IN ('open', 'acknowledged');

CREATE TRIGGER trg_runtime_alerts_touch_updated_at
BEFORE UPDATE ON runtime_alerts
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
