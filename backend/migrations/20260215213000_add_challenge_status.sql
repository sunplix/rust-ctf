BEGIN;

ALTER TABLE challenges
  ADD COLUMN status VARCHAR(16) NOT NULL DEFAULT 'draft'
  CHECK (status IN ('draft', 'published', 'offline'));

UPDATE challenges
SET status = CASE
  WHEN is_visible THEN 'published'
  ELSE 'draft'
END;

CREATE INDEX idx_challenges_status ON challenges (status);

UPDATE challenge_versions
SET snapshot = snapshot || jsonb_build_object(
  'status',
  CASE
    WHEN COALESCE((snapshot ->> 'is_visible')::boolean, false) THEN 'published'
    ELSE 'draft'
  END
)
WHERE NOT (snapshot ? 'status');

COMMIT;
