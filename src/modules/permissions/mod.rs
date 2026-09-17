//! Role and permission assignment (RBAC persistence).
//!
//! The permission *catalogue* (`Permission`) lives in `shared::auth` because every
//! module checks permissions; this module owns the `roles`, `permissions`,
//! `role_permissions` and `user_roles` tables.
//!
//! TODO: admin API to list roles and grant/revoke user roles (`role.manage`),
//! writing an audit record for every change.

mod repository;

pub use repository::load_permissions_for_user;
