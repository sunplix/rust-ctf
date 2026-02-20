BEGIN;

INSERT INTO challenge_categories (slug, display_name, sort_order, is_builtin)
VALUES ('pwn', 'pwn', 65, TRUE)
ON CONFLICT ((LOWER(slug))) DO UPDATE
SET
  display_name = EXCLUDED.display_name,
  sort_order = LEAST(challenge_categories.sort_order, EXCLUDED.sort_order),
  is_builtin = TRUE;

COMMIT;
