mod errors;
mod status;

pub use errors::OrderError;
pub use status::OrderStatus;

crate::typed_id!(
    /// Identifier of a row in `orders`.
    OrderId
);
