-- Customer requirements (product sourcing requests and general business requirements).
-- TODO: add product_categories / products and link requirements.category_id when
-- the products module migration lands.

CREATE TABLE requirements (
    id                     UUID          PRIMARY KEY,
    requirement_number     TEXT          NOT NULL UNIQUE,
    customer_id            UUID          NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
    kind                   TEXT          NOT NULL CHECK (kind IN ('PRODUCT', 'GENERAL')),
    title                  TEXT          NOT NULL CHECK (char_length(title) BETWEEN 3 AND 200),
    description            TEXT          NOT NULL,
    quantity               NUMERIC(19,4) CHECK (quantity > 0),
    unit_of_measure        TEXT,
    target_unit_price      NUMERIC(19,4) CHECK (target_unit_price >= 0),
    target_currency        CHAR(3)       REFERENCES currencies (code),
    destination_country    CHAR(2)       NOT NULL REFERENCES countries (code),
    preferred_origin       CHAR(2)       REFERENCES countries (code),
    required_by            DATE,
    status                 TEXT          NOT NULL DEFAULT 'SUBMITTED'
                                         CHECK (status IN ('SUBMITTED', 'UNDER_REVIEW', 'SOURCING',
                                                           'QUOTED', 'CLOSED', 'CANCELLED')),
    assigned_to            UUID          REFERENCES users (id) ON DELETE SET NULL,
    created_by             UUID          NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at             TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ   NOT NULL DEFAULT now(),
    CHECK (target_unit_price IS NULL OR target_currency IS NOT NULL),
    CHECK (kind <> 'PRODUCT' OR (quantity IS NOT NULL AND unit_of_measure IS NOT NULL))
);

CREATE INDEX requirements_by_customer ON requirements (customer_id, created_at DESC);
CREATE INDEX requirements_by_status ON requirements (status, created_at DESC);
CREATE INDEX requirements_by_assignee ON requirements (assigned_to) WHERE assigned_to IS NOT NULL;

CREATE TRIGGER requirements_set_updated_at BEFORE UPDATE ON requirements
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE requirement_number_seq;
