-- Payment schedules (what is owed and when) and payments (money actually moving).
-- TODO: invoices and receipts tables (invoices module) referencing orders/payments.

CREATE TABLE payment_schedules (
    id                 UUID          PRIMARY KEY,
    order_id           UUID          NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    installment_number INTEGER       NOT NULL CHECK (installment_number > 0),
    payment_type       TEXT          NOT NULL CHECK (payment_type IN ('FULL', 'PARTIAL', 'ADVANCE', 'MILESTONE')),
    description        TEXT          NOT NULL,
    amount_due         NUMERIC(19,4) NOT NULL CHECK (amount_due > 0),
    amount_paid        NUMERIC(19,4) NOT NULL DEFAULT 0 CHECK (amount_paid >= 0),
    currency           CHAR(3)       NOT NULL REFERENCES currencies (code),
    due_date           DATE,
    status             TEXT          NOT NULL DEFAULT 'PENDING'
                                     CHECK (status IN ('PENDING', 'PARTIALLY_PAID', 'PAID', 'REFUNDED', 'CANCELLED')),
    created_at         TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (order_id, installment_number)
);

CREATE TRIGGER payment_schedules_set_updated_at BEFORE UPDATE ON payment_schedules
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE payments (
    id                UUID          PRIMARY KEY,
    payment_number    TEXT          NOT NULL UNIQUE,
    order_id          UUID          NOT NULL REFERENCES orders (id) ON DELETE RESTRICT,
    schedule_id       UUID          REFERENCES payment_schedules (id) ON DELETE RESTRICT,
    customer_id       UUID          NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
    payment_type      TEXT          NOT NULL CHECK (payment_type IN ('FULL', 'PARTIAL', 'ADVANCE', 'MILESTONE')),
    method            TEXT          NOT NULL CHECK (method IN ('BANK_TRANSFER', 'CARD', 'CASH', 'GATEWAY')),
    amount            NUMERIC(19,4) NOT NULL CHECK (amount > 0),
    currency          CHAR(3)       NOT NULL REFERENCES currencies (code),
    status            TEXT          NOT NULL DEFAULT 'PENDING'
                                    CHECK (status IN ('PENDING', 'PAID', 'FAILED', 'REFUNDED')),
    provider          TEXT,
    provider_reference TEXT,
    -- Client-supplied `Idempotency-Key` for manual recording; duplicates return the original.
    idempotency_key   TEXT          UNIQUE,
    failure_reason    TEXT,
    recorded_by       UUID          REFERENCES users (id) ON DELETE SET NULL,
    confirmed_by      UUID          REFERENCES users (id) ON DELETE SET NULL,
    confirmed_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_reference),
    CHECK (status <> 'PAID' OR confirmed_at IS NOT NULL)
);

CREATE INDEX payments_by_order ON payments (order_id, created_at DESC);
CREATE INDEX payments_by_customer ON payments (customer_id, created_at DESC);
CREATE INDEX payments_by_status ON payments (status, created_at DESC);

CREATE TRIGGER payments_set_updated_at BEFORE UPDATE ON payments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE payment_number_seq;

-- Raw inbound gateway webhooks. The UNIQUE constraint makes webhook handling
-- idempotent: a redelivered event is recorded once and processed once.
CREATE TABLE payment_gateway_events (
    id                UUID        PRIMARY KEY,
    provider          TEXT        NOT NULL,
    provider_event_id TEXT        NOT NULL,
    event_type        TEXT        NOT NULL,
    payload           JSONB       NOT NULL,
    received_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at      TIMESTAMPTZ,
    processing_error  TEXT,
    UNIQUE (provider, provider_event_id)
);

CREATE INDEX payment_gateway_events_unprocessed ON payment_gateway_events (received_at)
    WHERE processed_at IS NULL;
