BEGIN;

ALTER TABLE users
  ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN email_verified_at TIMESTAMPTZ;

UPDATE users
SET email_verified = TRUE,
    email_verified_at = COALESCE(email_verified_at, created_at);

CREATE INDEX idx_users_email_verified ON users (email_verified);

CREATE TABLE auth_email_verification_tokens (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used_at TIMESTAMPTZ,
  request_ip VARCHAR(80),
  user_agent TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX uk_auth_email_verification_tokens_hash
  ON auth_email_verification_tokens (token_hash);
CREATE INDEX idx_auth_email_verification_tokens_user_state
  ON auth_email_verification_tokens (user_id, used_at, expires_at DESC);

CREATE TABLE auth_password_reset_tokens (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used_at TIMESTAMPTZ,
  request_ip VARCHAR(80),
  user_agent TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX uk_auth_password_reset_tokens_hash
  ON auth_password_reset_tokens (token_hash);
CREATE INDEX idx_auth_password_reset_tokens_user_state
  ON auth_password_reset_tokens (user_id, used_at, expires_at DESC);

COMMIT;
