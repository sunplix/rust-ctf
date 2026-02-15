BEGIN;

ALTER TABLE challenges
  ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}'::text[],
  ADD COLUMN writeup_visibility VARCHAR(24) NOT NULL DEFAULT 'hidden'
    CHECK (writeup_visibility IN ('hidden', 'after_solve', 'after_contest', 'public')),
  ADD COLUMN writeup_content TEXT NOT NULL DEFAULT '',
  ADD COLUMN current_version INTEGER NOT NULL DEFAULT 1 CHECK (current_version >= 1);

CREATE TABLE challenge_versions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  version_no INTEGER NOT NULL CHECK (version_no >= 1),
  snapshot JSONB NOT NULL,
  change_note TEXT NOT NULL DEFAULT '',
  created_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (challenge_id, version_no)
);

CREATE INDEX idx_challenge_versions_challenge_version
  ON challenge_versions (challenge_id, version_no DESC);

CREATE TABLE challenge_attachments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  filename VARCHAR(255) NOT NULL,
  content_type VARCHAR(128) NOT NULL DEFAULT 'application/octet-stream',
  storage_path TEXT NOT NULL,
  size_bytes BIGINT NOT NULL CHECK (size_bytes >= 0),
  uploaded_by UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_challenge_attachments_challenge
  ON challenge_attachments (challenge_id, created_at DESC);

INSERT INTO challenge_versions (challenge_id, version_no, snapshot, change_note, created_by)
SELECT c.id,
       1,
       jsonb_build_object(
         'title', c.title,
         'slug', c.slug,
         'category', c.category,
         'difficulty', c.difficulty,
         'description', c.description,
         'static_score', c.static_score,
         'min_score', c.min_score,
         'max_score', c.max_score,
         'challenge_type', c.challenge_type,
         'flag_mode', c.flag_mode,
         'flag_hash', c.flag_hash,
         'compose_template', c.compose_template,
         'metadata', c.metadata,
         'is_visible', c.is_visible,
         'tags', c.tags,
         'writeup_visibility', c.writeup_visibility,
         'writeup_content', c.writeup_content
       ),
       'initial snapshot',
       c.created_by
FROM challenges c
ON CONFLICT (challenge_id, version_no) DO NOTHING;

COMMIT;
