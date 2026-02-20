BEGIN;

CREATE TABLE challenge_categories (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  slug VARCHAR(32) NOT NULL,
  display_name VARCHAR(64) NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (char_length(slug) BETWEEN 1 AND 32),
  CHECK (char_length(display_name) BETWEEN 1 AND 64)
);

CREATE UNIQUE INDEX uk_challenge_categories_slug_lower
  ON challenge_categories ((LOWER(slug)));

CREATE INDEX idx_challenge_categories_sort
  ON challenge_categories (sort_order ASC, created_at ASC);

CREATE TRIGGER trg_challenge_categories_touch_updated_at
BEFORE UPDATE ON challenge_categories
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

INSERT INTO challenge_categories (slug, display_name, sort_order, is_builtin)
VALUES
  ('misc', 'misc', 10, TRUE),
  ('crypto', 'crypto', 20, TRUE),
  ('web', 'web', 30, TRUE),
  ('reverse', 'reverse', 40, TRUE),
  ('mobile', 'mobile', 50, TRUE),
  ('osint', 'osint', 60, TRUE),
  ('penetration', 'penetration', 70, TRUE)
ON CONFLICT ((LOWER(slug))) DO NOTHING;

INSERT INTO challenge_categories (slug, display_name, sort_order, is_builtin)
SELECT DISTINCT
  LOWER(BTRIM(c.category)) AS slug,
  LOWER(BTRIM(c.category)) AS display_name,
  100 AS sort_order,
  FALSE AS is_builtin
FROM challenges c
WHERE BTRIM(c.category) <> ''
ON CONFLICT ((LOWER(slug))) DO NOTHING;

ALTER TABLE contests
  ADD COLUMN first_blood_bonus_percent INTEGER NOT NULL DEFAULT 10
    CHECK (first_blood_bonus_percent >= 0 AND first_blood_bonus_percent <= 500),
  ADD COLUMN second_blood_bonus_percent INTEGER NOT NULL DEFAULT 5
    CHECK (second_blood_bonus_percent >= 0 AND second_blood_bonus_percent <= 500),
  ADD COLUMN third_blood_bonus_percent INTEGER NOT NULL DEFAULT 2
    CHECK (third_blood_bonus_percent >= 0 AND third_blood_bonus_percent <= 500);

COMMIT;
