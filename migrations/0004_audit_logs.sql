-- Append-only audit trail. Written in the same transaction as the change it records.

CREATE TABLE audit_logs (
    id            UUID        PRIMARY KEY,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_type    TEXT        NOT NULL CHECK (actor_type IN ('USER', 'SYSTEM', 'EXTERNAL')),
    actor_user_id UUID        REFERENCES users (id) ON DELETE SET NULL,
    action        TEXT        NOT NULL CHECK (action ~ '^[a-z_]+(\.[a-z_]+)+$'),
    entity_type   TEXT        NOT NULL,
    entity_id     UUID,
    request_id    TEXT,
    ip_address    INET,
    -- Flexible, action-specific context (e.g. from/to status). Never secrets or tokens.
    metadata      JSONB       NOT NULL DEFAULT '{}'::jsonb,
    CHECK (actor_type <> 'USER' OR actor_user_id IS NOT NULL)
);

CREATE INDEX audit_logs_by_entity ON audit_logs (entity_type, entity_id, occurred_at DESC);
CREATE INDEX audit_logs_by_actor ON audit_logs (actor_user_id, occurred_at DESC);
CREATE INDEX audit_logs_by_time ON audit_logs (occurred_at DESC);

CREATE OR REPLACE FUNCTION prevent_audit_log_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_logs is append-only';
END;
$$;

CREATE TRIGGER audit_logs_append_only BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_log_mutation();
