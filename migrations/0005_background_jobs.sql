-- Durable job queue / transactional outbox.
--
-- Business workflows insert jobs in the SAME transaction as their data changes, so
-- a committed quotation acceptance always has its notification job, and a rolled
-- back one never does. Workers claim jobs with FOR UPDATE SKIP LOCKED.

CREATE TABLE background_jobs (
    id              UUID        PRIMARY KEY,
    queue           TEXT        NOT NULL DEFAULT 'default',
    kind            TEXT        NOT NULL,
    payload         JSONB       NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'PENDING'
                                CHECK (status IN ('PENDING', 'RUNNING', 'SUCCEEDED', 'DEAD')),
    attempts        INTEGER     NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts    INTEGER     NOT NULL DEFAULT 10 CHECK (max_attempts > 0),
    run_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_at       TIMESTAMPTZ,
    last_error      TEXT,
    -- Optional de-duplication key, e.g. 'invoice.generate:<order_id>'.
    idempotency_key TEXT        UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX background_jobs_ready ON background_jobs (queue, run_at) WHERE status = 'PENDING';
CREATE INDEX background_jobs_running ON background_jobs (locked_at) WHERE status = 'RUNNING';
