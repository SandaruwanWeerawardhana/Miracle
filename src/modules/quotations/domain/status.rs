use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuotationStatus {
    Draft,
    Sent,
    Accepted,
    Rejected,
    Expired,
    Cancelled,
}

impl QuotationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Sent => "SENT",
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::Expired => "EXPIRED",
            Self::Cancelled => "CANCELLED",
        }
    }

    /// ```text
    /// DRAFT ─► SENT ─► ACCEPTED
    ///   │        ├───► REJECTED
    ///   │        ├───► EXPIRED
    ///   └────────┴───► CANCELLED
    /// ```
    pub fn can_transition_to(self, next: Self) -> bool {
        use QuotationStatus::*;
        matches!(
            (self, next),
            (Draft, Sent | Cancelled) | (Sent, Accepted | Rejected | Expired | Cancelled)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Rejected | Self::Expired | Self::Cancelled
        )
    }
}

impl fmt::Display for QuotationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for QuotationStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "DRAFT" => Ok(Self::Draft),
            "SENT" => Ok(Self::Sent),
            "ACCEPTED" => Ok(Self::Accepted),
            "REJECTED" => Ok(Self::Rejected),
            "EXPIRED" => Ok(Self::Expired),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(format!("unknown quotation status `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QuotationStatus::*;

    #[test]
    fn terminal_states_have_no_transitions() {
        for terminal in [Accepted, Rejected, Expired, Cancelled] {
            assert!(terminal.is_terminal());
            for next in [Draft, Sent, Accepted, Rejected, Expired, Cancelled] {
                assert!(!terminal.can_transition_to(next));
            }
        }
    }

    #[test]
    fn draft_cannot_be_accepted_directly() {
        assert!(!Draft.can_transition_to(Accepted));
        assert!(Sent.can_transition_to(Accepted));
    }
}
