-- Profiles attached to user accounts.
-- TODO: when customers become multi-user organisations, add `customer_members`
-- instead of changing the 1:1 `customers.user_id` relationship in place.

CREATE TABLE customers (
    id                           UUID        PRIMARY KEY,
    user_id                      UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE RESTRICT,
    customer_number              TEXT        NOT NULL UNIQUE,
    company_name                 TEXT,
    business_registration_number TEXT,
    country_code                 CHAR(2)     NOT NULL REFERENCES countries (code),
    city                         TEXT,
    address_line1                TEXT,
    address_line2                TEXT,
    postal_code                  TEXT,
    preferred_currency           CHAR(3)     NOT NULL DEFAULT 'LKR' REFERENCES currencies (code),
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER customers_set_updated_at BEFORE UPDATE ON customers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE customer_number_seq;

CREATE TABLE suppliers (
    id                  UUID        PRIMARY KEY,
    -- Nullable: procurement staff may register suppliers that never log in.
    user_id             UUID        UNIQUE REFERENCES users (id) ON DELETE SET NULL,
    supplier_number     TEXT        NOT NULL UNIQUE,
    legal_name          TEXT        NOT NULL,
    trading_name        TEXT,
    country_code        CHAR(2)     NOT NULL REFERENCES countries (code),
    contact_email       CITEXT,
    contact_phone       TEXT,
    website             TEXT,
    verification_status TEXT        NOT NULL DEFAULT 'UNVERIFIED'
                                    CHECK (verification_status IN
                                        ('UNVERIFIED', 'PENDING_REVIEW', 'VERIFIED', 'REJECTED', 'SUSPENDED')),
    verified_at         TIMESTAMPTZ,
    created_by          UUID        REFERENCES users (id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (verification_status <> 'VERIFIED' OR verified_at IS NOT NULL)
);

CREATE INDEX suppliers_by_country_status ON suppliers (country_code, verification_status);

CREATE TRIGGER suppliers_set_updated_at BEFORE UPDATE ON suppliers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE supplier_number_seq;

-- Full history of verification decisions (documents are linked via the documents module).
CREATE TABLE supplier_verification_reviews (
    id          UUID        PRIMARY KEY,
    supplier_id UUID        NOT NULL REFERENCES suppliers (id) ON DELETE CASCADE,
    from_status TEXT        NOT NULL,
    to_status   TEXT        NOT NULL,
    notes       TEXT,
    reviewed_by UUID        NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    reviewed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX supplier_verification_reviews_by_supplier
    ON supplier_verification_reviews (supplier_id, reviewed_at DESC);

CREATE TABLE staff_members (
    id              UUID        PRIMARY KEY,
    user_id         UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE RESTRICT,
    employee_number TEXT        NOT NULL UNIQUE,
    department      TEXT        NOT NULL,
    job_title       TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER staff_members_set_updated_at BEFORE UPDATE ON staff_members
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
