-- Wholesale orders and their status timeline (order tracking).
-- TODO: shipments/import records (logistics & imports modules) reference orders.

CREATE TABLE orders (
    id                  UUID          PRIMARY KEY,
    order_number        TEXT          NOT NULL UNIQUE,
    customer_id         UUID          NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
    -- UNIQUE makes quotation acceptance idempotent: one order per quotation.
    quotation_id        UUID          UNIQUE REFERENCES quotations (id) ON DELETE RESTRICT,
    currency            CHAR(3)       NOT NULL REFERENCES currencies (code),
    status              TEXT          NOT NULL DEFAULT 'PENDING'
                                      CHECK (status IN ('PENDING', 'CONFIRMED', 'PROCESSING', 'PROCUREMENT',
                                                        'IMPORTING', 'SHIPPED', 'OUT_FOR_DELIVERY',
                                                        'DELIVERED', 'CANCELLED')),
    subtotal            NUMERIC(19,4) NOT NULL CHECK (subtotal >= 0),
    charges_total       NUMERIC(19,4) NOT NULL,
    grand_total         NUMERIC(19,4) NOT NULL CHECK (grand_total >= 0),
    cancellation_reason TEXT,
    created_by          UUID          NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    version             INTEGER       NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ   NOT NULL DEFAULT now(),
    CHECK (status <> 'CANCELLED' OR cancellation_reason IS NOT NULL)
);

CREATE INDEX orders_by_customer ON orders (customer_id, created_at DESC);
CREATE INDEX orders_by_status ON orders (status, created_at DESC);

CREATE TRIGGER orders_set_updated_at BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE order_number_seq;

CREATE TABLE order_items (
    id              UUID          PRIMARY KEY,
    order_id        UUID          NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    line_number     INTEGER       NOT NULL CHECK (line_number > 0),
    product_id      UUID,
    supplier_id     UUID          REFERENCES suppliers (id) ON DELETE RESTRICT,
    description     TEXT          NOT NULL,
    quantity        NUMERIC(19,4) NOT NULL CHECK (quantity > 0),
    unit_of_measure TEXT          NOT NULL,
    unit_price      NUMERIC(19,4) NOT NULL CHECK (unit_price >= 0),
    line_total      NUMERIC(19,4) NOT NULL CHECK (line_total >= 0),
    UNIQUE (order_id, line_number)
);

CREATE INDEX order_items_by_supplier ON order_items (supplier_id) WHERE supplier_id IS NOT NULL;

CREATE TABLE order_status_history (
    id          UUID        PRIMARY KEY,
    order_id    UUID        NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    from_status TEXT,
    to_status   TEXT        NOT NULL,
    note        TEXT,
    -- Whether the entry is shown on the customer's tracking timeline.
    is_public   BOOLEAN     NOT NULL DEFAULT TRUE,
    changed_by  UUID        REFERENCES users (id) ON DELETE SET NULL,
    changed_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX order_status_history_by_order ON order_status_history (order_id, changed_at);
