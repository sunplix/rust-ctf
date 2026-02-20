BEGIN;

CREATE TABLE site_settings (
  id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
  site_name VARCHAR(80) NOT NULL DEFAULT 'RUST CTF',
  site_subtitle VARCHAR(160) NOT NULL DEFAULT '竞赛平台',
  home_title VARCHAR(160) NOT NULL DEFAULT '欢迎来到 Rust CTF',
  home_tagline TEXT NOT NULL DEFAULT '面向实战的安全竞赛平台，专注比赛与协作。',
  home_signature VARCHAR(200) NOT NULL DEFAULT 'Think clearly. Ship securely.',
  footer_text VARCHAR(240) NOT NULL DEFAULT '© 2026 Rust-CTF. All rights reserved.',
  updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO site_settings (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE TRIGGER trg_site_settings_touch_updated_at
BEFORE UPDATE ON site_settings
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

COMMIT;
