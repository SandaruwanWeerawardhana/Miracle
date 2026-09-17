-- RBAC: users have roles, roles have permissions. Authorization checks use permissions only.
-- Permission codes must stay in sync with `modules::permissions::domain::Permission`.
--
-- Scope convention: a `.own` suffix restricts the capability to resources owned by
-- the caller's customer/supplier profile; the unsuffixed code grants access to all.

CREATE TABLE roles (
    code        TEXT        PRIMARY KEY CHECK (code ~ '^[A-Z][A-Z_]*$'),
    name        TEXT        NOT NULL,
    description TEXT,
    is_system   BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE permissions (
    code        TEXT PRIMARY KEY CHECK (code ~ '^[a-z_]+(\.[a-z_]+)+$'),
    description TEXT NOT NULL
);

CREATE TABLE role_permissions (
    role_code       TEXT NOT NULL REFERENCES roles (code) ON DELETE CASCADE,
    permission_code TEXT NOT NULL REFERENCES permissions (code) ON DELETE CASCADE,
    PRIMARY KEY (role_code, permission_code)
);

CREATE TABLE user_roles (
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_code  TEXT        NOT NULL REFERENCES roles (code) ON DELETE RESTRICT,
    granted_by UUID        REFERENCES users (id) ON DELETE SET NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_code)
);

CREATE INDEX user_roles_by_role ON user_roles (role_code);

INSERT INTO roles (code, name, is_system) VALUES
    ('CUSTOMER', 'Customer', TRUE),
    ('SUPPLIER', 'Supplier', TRUE),
    ('SALES_STAFF', 'Sales staff', TRUE),
    ('PROCUREMENT_STAFF', 'Procurement staff', TRUE),
    ('FINANCE_STAFF', 'Finance staff', TRUE),
    ('LOGISTICS_STAFF', 'Logistics staff', TRUE),
    ('TRAVEL_STAFF', 'Travel staff', TRUE),
    ('IT_STAFF', 'IT staff', TRUE),
    ('BUSINESS_CONSULTANT', 'Business consultant', TRUE),
    ('MANAGER', 'Manager', TRUE),
    ('ADMIN', 'Administrator', TRUE),
    ('SUPER_ADMIN', 'Super administrator', TRUE);

INSERT INTO permissions (code, description) VALUES
    ('user.read', 'View user accounts'),
    ('user.manage', 'Create, disable and update user accounts'),
    ('role.manage', 'Assign roles and change role permissions'),
    ('customer.read', 'View all customer profiles'),
    ('customer.update', 'Update any customer profile'),
    ('supplier.read', 'View suppliers'),
    ('supplier.create', 'Register suppliers'),
    ('supplier.update', 'Update suppliers'),
    ('supplier.verify', 'Approve, reject or suspend supplier verification'),
    ('product.read', 'View products'),
    ('product.manage', 'Create and update products'),
    ('requirement.create', 'Submit requirements'),
    ('requirement.read.own', 'View own requirements'),
    ('requirement.read', 'View all requirements'),
    ('requirement.update', 'Update and assign requirements'),
    ('sourcing.manage', 'Run sourcing and request supplier quotations'),
    ('supplier_quotation.submit.own', 'Submit quotations as a supplier'),
    ('supplier_quotation.read', 'View supplier quotations'),
    ('quotation.create', 'Create customer quotations'),
    ('quotation.read.own', 'View own quotations'),
    ('quotation.read', 'View all quotations'),
    ('quotation.update', 'Edit draft quotations'),
    ('quotation.send', 'Send quotations to customers'),
    ('quotation.respond.own', 'Accept or reject own quotations'),
    ('order.create', 'Create orders manually'),
    ('order.read.own', 'View own orders'),
    ('order.read', 'View all orders'),
    ('order.update', 'Update orders and advance order status'),
    ('order.cancel', 'Cancel orders'),
    ('shipment.read', 'View shipments'),
    ('shipment.manage', 'Manage imports, shipments and tracking'),
    ('payment.read.own', 'View own payments'),
    ('payment.read', 'View all payments'),
    ('payment.record', 'Record offline payments'),
    ('payment.confirm', 'Confirm payments'),
    ('payment.refund', 'Refund payments'),
    ('invoice.read.own', 'View own invoices'),
    ('invoice.read', 'View all invoices'),
    ('invoice.manage', 'Issue and void invoices'),
    ('document.upload', 'Upload documents'),
    ('document.read', 'View all documents'),
    ('document.manage', 'Delete documents and change visibility'),
    ('support.ticket.create', 'Open support tickets'),
    ('support.ticket.manage', 'Handle support tickets'),
    ('it_request.manage', 'Handle IT service requests'),
    ('cms.read', 'View CMS drafts'),
    ('cms.write', 'Edit CMS content'),
    ('cms.publish', 'Publish CMS content'),
    ('report.operational.read', 'View operational reports'),
    ('report.financial.read', 'View financial reports'),
    ('audit.read', 'View audit logs'),
    ('settings.manage', 'Change application settings');

INSERT INTO role_permissions (role_code, permission_code)
SELECT role_code, permission_code
FROM (VALUES
    ('CUSTOMER', ARRAY[
        'requirement.create', 'requirement.read.own', 'quotation.read.own', 'quotation.respond.own',
        'order.read.own', 'payment.read.own', 'invoice.read.own', 'document.upload',
        'support.ticket.create', 'product.read'
    ]),
    ('SUPPLIER', ARRAY[
        'supplier_quotation.submit.own', 'document.upload', 'support.ticket.create', 'product.read'
    ]),
    ('SALES_STAFF', ARRAY[
        'customer.read', 'requirement.read', 'requirement.update', 'quotation.create', 'quotation.read',
        'quotation.update', 'quotation.send', 'order.read', 'product.read', 'supplier_quotation.read',
        'document.read', 'document.upload'
    ]),
    ('PROCUREMENT_STAFF', ARRAY[
        'supplier.read', 'supplier.create', 'supplier.update', 'requirement.read', 'sourcing.manage',
        'supplier_quotation.read', 'product.read', 'product.manage', 'order.read', 'document.read',
        'document.upload'
    ]),
    ('FINANCE_STAFF', ARRAY[
        'order.read', 'payment.read', 'payment.record', 'payment.confirm', 'invoice.read',
        'invoice.manage', 'report.financial.read', 'document.read'
    ]),
    ('LOGISTICS_STAFF', ARRAY[
        'order.read', 'order.update', 'shipment.read', 'shipment.manage', 'document.read', 'document.upload'
    ]),
    ('TRAVEL_STAFF', ARRAY['customer.read', 'document.read', 'support.ticket.manage']),
    ('IT_STAFF', ARRAY['customer.read', 'it_request.manage', 'support.ticket.manage']),
    ('BUSINESS_CONSULTANT', ARRAY['customer.read', 'requirement.read', 'document.read']),
    ('MANAGER', ARRAY[
        'user.read', 'customer.read', 'supplier.read', 'supplier.verify', 'requirement.read',
        'quotation.read', 'quotation.send', 'order.read', 'order.update', 'order.cancel', 'shipment.read',
        'payment.read', 'payment.refund', 'invoice.read', 'document.read', 'support.ticket.manage',
        'report.operational.read', 'report.financial.read', 'audit.read', 'cms.read', 'cms.publish'
    ])
) AS grants (role_code, codes)
CROSS JOIN LATERAL unnest(codes) AS permission_code;

-- ADMIN receives everything except role management; SUPER_ADMIN receives everything.
INSERT INTO role_permissions (role_code, permission_code)
SELECT 'ADMIN', code FROM permissions WHERE code <> 'role.manage';

INSERT INTO role_permissions (role_code, permission_code)
SELECT 'SUPER_ADMIN', code FROM permissions;
