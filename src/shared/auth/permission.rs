use std::{collections::HashSet, fmt, str::FromStr};

/// Declares the permission catalogue once: variant, code string and iteration list.
/// Codes must match `migrations/*_roles_and_permissions.sql`.
macro_rules! permissions {
    ($($variant:ident => $code:literal),+ $(,)?) => {
        /// A granular capability. Authorization checks always use permissions,
        /// never role names.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Permission {
            $($variant),+
        }

        impl Permission {
            pub const ALL: &'static [Permission] = &[$(Permission::$variant),+];

            pub fn code(self) -> &'static str {
                match self {
                    $(Permission::$variant => $code),+
                }
            }
        }

        impl FromStr for Permission {
            type Err = UnknownPermission;

            fn from_str(code: &str) -> Result<Self, Self::Err> {
                match code {
                    $($code => Ok(Permission::$variant),)+
                    other => Err(UnknownPermission(other.to_owned())),
                }
            }
        }
    };
}

permissions! {
    UserRead => "user.read",
    UserManage => "user.manage",
    RoleManage => "role.manage",
    CustomerRead => "customer.read",
    CustomerUpdate => "customer.update",
    SupplierRead => "supplier.read",
    SupplierCreate => "supplier.create",
    SupplierUpdate => "supplier.update",
    SupplierVerify => "supplier.verify",
    ProductRead => "product.read",
    ProductManage => "product.manage",
    RequirementCreate => "requirement.create",
    RequirementReadOwn => "requirement.read.own",
    RequirementRead => "requirement.read",
    RequirementUpdate => "requirement.update",
    SourcingManage => "sourcing.manage",
    SupplierQuotationSubmitOwn => "supplier_quotation.submit.own",
    SupplierQuotationRead => "supplier_quotation.read",
    QuotationCreate => "quotation.create",
    QuotationReadOwn => "quotation.read.own",
    QuotationRead => "quotation.read",
    QuotationUpdate => "quotation.update",
    QuotationSend => "quotation.send",
    QuotationRespondOwn => "quotation.respond.own",
    OrderCreate => "order.create",
    OrderReadOwn => "order.read.own",
    OrderRead => "order.read",
    OrderUpdate => "order.update",
    OrderCancel => "order.cancel",
    ShipmentRead => "shipment.read",
    ShipmentManage => "shipment.manage",
    PaymentReadOwn => "payment.read.own",
    PaymentRead => "payment.read",
    PaymentRecord => "payment.record",
    PaymentConfirm => "payment.confirm",
    PaymentRefund => "payment.refund",
    InvoiceReadOwn => "invoice.read.own",
    InvoiceRead => "invoice.read",
    InvoiceManage => "invoice.manage",
    DocumentUpload => "document.upload",
    DocumentRead => "document.read",
    DocumentManage => "document.manage",
    SupportTicketCreate => "support.ticket.create",
    SupportTicketManage => "support.ticket.manage",
    ItRequestManage => "it_request.manage",
    CmsRead => "cms.read",
    CmsWrite => "cms.write",
    CmsPublish => "cms.publish",
    ReportOperationalRead => "report.operational.read",
    ReportFinancialRead => "report.financial.read",
    AuditRead => "audit.read",
    SettingsManage => "settings.manage",
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown permission code `{0}`")]
pub struct UnknownPermission(pub String);

/// The effective permissions of an authenticated principal.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet(HashSet<Permission>);

impl PermissionSet {
    /// Unknown codes (e.g. added in the database before a deploy) are skipped, not fatal.
    pub fn from_codes<I, S>(codes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let set = codes
            .into_iter()
            .filter_map(|code| match code.as_ref().parse::<Permission>() {
                Ok(permission) => Some(permission),
                Err(error) => {
                    tracing::warn!(%error, "ignoring unknown permission");
                    None
                }
            })
            .collect();
        Self(set)
    }

    pub fn contains(&self, permission: Permission) -> bool {
        self.0.contains(&permission)
    }

    pub fn contains_any(&self, permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .any(|permission| self.contains(*permission))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_round_trip() {
        let mut seen = HashSet::new();
        for permission in Permission::ALL {
            assert!(
                seen.insert(permission.code()),
                "duplicate code {}",
                permission.code()
            );
            assert_eq!(
                permission.code().parse::<Permission>().unwrap(),
                *permission
            );
        }
    }

    #[test]
    fn unknown_codes_are_ignored() {
        let set = PermissionSet::from_codes(["quotation.read", "does.not_exist"]);
        assert!(set.contains(Permission::QuotationRead));
        assert!(!set.contains(Permission::QuotationSend));
    }
}
