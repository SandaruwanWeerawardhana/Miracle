-- Customer quotations. All lines and charges share the quotation currency.
-- TODO: supplier_quotations (sourcing module) and the FK from quotation_items.

CREATE TABLE quotations (
    id               UUID          PRIMARY KEY,
    quotation_number TEXT          NOT NULL UNIQUE,
    customer_id      UUID          NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
    requirement_id   UUID          REFERENCES requirements (id) ON DELETE RESTRICT,
    currency         CHAR(3)       NOT NULL REFERENCES currencies (code),
    status           TEXT          NOT NULL DEFAULT 'DRAFT'
                                   CHECK (status IN ('DRAFT', 'SENT', 'ACCEPTED', 'REJECTED', 'EXPIRED', 'CANCELLED')),
    subtotal         NUMERIC(19,4) NOT NULL DEFAULT 0 CHECK (subtotal >= 0),
    charges_total    NUMERIC(19,4) NOT NULL DEFAULT 0,
    grand_total      NUMERIC(19,4) NOT NULL DEFAULT 0 CHECK (grand_total >= 0),
    valid_until      TIMESTAMPTZ,
    terms            TEXT,
    notes            TEXT,
    sent_at          TIMESTAMPTZ,
    responded_at     TIMESTAMPTZ,
    rejection_reason TEXT,
    created_by       UUID          NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    -- Optimistic concurrency: updates must match the version they read.
    version          INTEGER       NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    CHECK (status = 'DRAFT' OR status = 'CANCELLED' OR (valid_until IS NOT NULL AND sent_at IS NOT NULL)),
    CHECK (status NOT IN ('ACCEPTED', 'REJECTED') OR responded_at IS NOT NULL)
);

CREATE INDEX quotations_by_customer ON quotations (customer_id, created_at DESC);
CREATE INDEX quotations_by_status ON quotations (status, created_at DESC);
CREATE INDEX quotations_by_requirement ON quotations (requirement_id) WHERE requirement_id IS NOT NULL;

CREATE TRIGGER quotations_set_updated_at BEFORE UPDATE ON quotations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE quotation_number_seq;

CREATE TABLE quotation_items (
    id              UUID          PRIMARY KEY,
    quotation_id    UUID          NOT NULL REFERENCES quotations (id) ON DELETE CASCADE,
    line_number     INTEGER       NOT NULL CHECK (line_number > 0),
    product_id      UUID,
    supplier_id     UUID          REFERENCES suppliers (id) ON DELETE RESTRICT,
    description     TEXT          NOT NULL,
    quantity        NUMERIC(19,4) NOT NULL CHECK (quantity > 0),
    unit_of_measure TEXT          NOT NULL,
    unit_price      NUMERIC(19,4) NOT NULL CHECK (unit_price >= 0),
    line_total      NUMERIC(19,4) NOT NULL CHECK (line_total >= 0),
    UNIQUE (quotation_id, line_number)
);

-- Taxes, shipping, fees and discounts. Discounts are stored as positive amounts.
CREATE TABLE quotation_charges (
    id           UUID          PRIMARY KEY,
    quotation_id UUID          NOT NULL REFERENCES quotations (id) ON DELETE CASCADE,
    kind         TEXT          NOT NULL CHECK (kind IN ('TAX', 'SHIPPING', 'FEE', 'DISCOUNT', 'OTHER')),
    description  TEXT          NOT NULL,
    amount       NUMERIC(19,4) NOT NULL CHECK (amount >= 0)
);

CREATE INDEX quotation_charges_by_quotation ON quotation_charges (quotation_id);
