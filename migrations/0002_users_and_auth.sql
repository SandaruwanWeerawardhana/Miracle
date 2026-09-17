CREATE TABLE users (
    id                    UUID        PRIMARY KEY,
    email                 CITEXT      NOT NULL UNIQUE,
    password_hash         TEXT        NOT NULL,
    full_name             TEXT        NOT NULL CHECK (char_length(full_name) BETWEEN 1 AND 200),
    phone                 TEXT,
    account_type          TEXT        NOT NULL CHECK (account_type IN ('CUSTOMER', 'SUPPLIER', 'STAFF')),
    status                TEXT        NOT NULL DEFAULT 'PENDING_VERIFICATION'
                                      CHECK (status IN ('PENDING_VERIFICATION', 'ACTIVE', 'DISABLED')),
    email_verified_at     TIMESTAMPTZ,
    failed_login_attempts INTEGER     NOT NULL DEFAULT 0 CHECK (failed_login_attempts >= 0),
    locked_until          TIMESTAMPTZ,
    last_login_at         TIMESTAMPTZ,
    preferred_language    TEXT        NOT NULL DEFAULT 'en',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- One row per login (device). Refresh tokens are stored hashed and rotated on use.
CREATE TABLE user_sessions (
    id                          UUID        PRIMARY KEY,
    user_id                     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    refresh_token_hash          BYTEA       NOT NULL UNIQUE,
    -- Hash of the previously issued refresh token. Presenting it again means the
    -- token was stolen/replayed, so the whole session is revoked.
    previous_refresh_token_hash BYTEA,
    user_agent                  TEXT,
    ip_address                  INET,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at                  TIMESTAMPTZ NOT NULL,
    revoked_at                  TIMESTAMPTZ,
    revoked_reason              TEXT
);

CREATE INDEX user_sessions_active_by_user ON user_sessions (user_id) WHERE revoked_at IS NULL;
CREATE INDEX user_sessions_previous_hash ON user_sessions (previous_refresh_token_hash)
    WHERE previous_refresh_token_hash IS NOT NULL;

-- Single-use tokens for email verification and password reset.
CREATE TABLE user_action_tokens (
    id          UUID        PRIMARY KEY,
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    purpose     TEXT        NOT NULL CHECK (purpose IN ('EMAIL_VERIFICATION', 'PASSWORD_RESET')),
    token_hash  BYTEA       NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX user_action_tokens_by_user ON user_action_tokens (user_id, purpose);
