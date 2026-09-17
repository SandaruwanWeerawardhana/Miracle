//! Strongly typed identifiers.
//!
//! Every externally visible entity uses a UUID v7 primary key (time-ordered,
//! index friendly, not guessable). A distinct Rust type per entity prevents
//! passing an `OrderId` where a `QuotationId` is expected.
//!
//! Each module declares its own IDs with this macro, e.g.
//! `crate::typed_id!(QuotationId);` in `modules/quotations/domain`.

#[macro_export]
macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            ::serde::Serialize, ::serde::Deserialize, ::sqlx::Type, ::utoipa::ToSchema,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        #[schema(value_type = String, format = Uuid)]
        pub struct $name(::uuid::Uuid);

        #[allow(dead_code, reason = "not every ID uses every helper")]
        impl $name {
            /// Generates a new time-ordered (v7) identifier.
            pub fn new() -> Self {
                Self(::uuid::Uuid::now_v7())
            }

            pub const fn from_uuid(uuid: ::uuid::Uuid) -> Self {
                Self(uuid)
            }

            pub const fn as_uuid(&self) -> ::uuid::Uuid {
                self.0
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::std::convert::From<::uuid::Uuid> for $name {
            fn from(uuid: ::uuid::Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

// The user identity is referenced by nearly every module (actors, `created_by`,
// ownership checks), so it is the one entity ID defined here.
typed_id!(
    /// Identifier of a row in `users`.
    UserId
);

#[cfg(test)]
mod tests {
    crate::typed_id!(ExampleId);

    #[test]
    fn ids_are_v7_and_ordered() {
        let first = ExampleId::new();
        let second = ExampleId::new();
        assert_eq!(first.as_uuid().get_version_num(), 7);
        assert!(first <= second);
    }
}
