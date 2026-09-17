use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Processing,
    Procurement,
    Importing,
    Shipped,
    OutForDelivery,
    Delivered,
    Cancelled,
}

impl OrderStatus {
    pub const ALL: [Self; 9] = [
        Self::Pending,
        Self::Confirmed,
        Self::Processing,
        Self::Procurement,
        Self::Importing,
        Self::Shipped,
        Self::OutForDelivery,
        Self::Delivered,
        Self::Cancelled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Confirmed => "CONFIRMED",
            Self::Processing => "PROCESSING",
            Self::Procurement => "PROCUREMENT",
            Self::Importing => "IMPORTING",
            Self::Shipped => "SHIPPED",
            Self::OutForDelivery => "OUT_FOR_DELIVERY",
            Self::Delivered => "DELIVERED",
            Self::Cancelled => "CANCELLED",
        }
    }

    /// The only allowed status changes. Anything else is rejected.
    ///
    /// ```text
    /// PENDING ─► CONFIRMED ─► PROCESSING ─► PROCUREMENT ─► IMPORTING ─► SHIPPED
    ///                              │              └──────────────────────► │
    ///                              └─────────────────────────────────────► │
    /// SHIPPED ─► OUT_FOR_DELIVERY ─► DELIVERED        SHIPPED ─► DELIVERED
    /// CANCELLED is reachable from PENDING, CONFIRMED, PROCESSING, PROCUREMENT
    /// (goods already importing or shipped cannot simply be cancelled).
    /// ```
    pub fn can_transition_to(self, next: Self) -> bool {
        use OrderStatus::*;
        matches!(
            (self, next),
            (Pending, Confirmed)
                | (Confirmed, Processing)
                | (Processing, Procurement | Shipped)
                | (Procurement, Importing | Shipped)
                | (Importing, Shipped)
                | (Shipped, OutForDelivery | Delivered)
                | (OutForDelivery, Delivered)
                | (Pending | Confirmed | Processing | Procurement, Cancelled)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Delivered | Self::Cancelled)
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OrderStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
            .ok_or_else(|| format!("unknown order status `{value}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::OrderStatus::{self, *};

    #[test]
    fn terminal_states_are_final() {
        for terminal in [Delivered, Cancelled] {
            assert!(
                OrderStatus::ALL
                    .iter()
                    .all(|next| !terminal.can_transition_to(*next))
            );
        }
    }

    #[test]
    fn cannot_skip_confirmation_or_cancel_in_transit() {
        assert!(!Pending.can_transition_to(Shipped));
        assert!(!Shipped.can_transition_to(Cancelled));
        assert!(Procurement.can_transition_to(Cancelled));
    }

    #[test]
    fn string_round_trip() {
        for status in OrderStatus::ALL {
            assert_eq!(status.as_str().parse::<OrderStatus>().unwrap(), status);
        }
    }
}
