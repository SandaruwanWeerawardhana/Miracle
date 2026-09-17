use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::{QuotationError, QuotationStatus};
use crate::{
    modules::{customers::CustomerId, suppliers::SupplierId},
    shared::money::{Currency, Money},
};

crate::typed_id!(
    /// Identifier of a row in `quotations`.
    QuotationId
);

#[derive(Debug, Clone)]
pub struct QuotationItem {
    pub line_number: i32,
    pub product_id: Option<Uuid>,
    pub supplier_id: Option<SupplierId>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_of_measure: String,
    pub unit_price: Money,
}

impl QuotationItem {
    pub fn line_total(&self) -> Result<Money, QuotationError> {
        Ok(self
            .unit_price
            .checked_mul(self.quantity)?
            .round_to_minor_units())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeKind {
    Tax,
    Shipping,
    Fee,
    Discount,
    Other,
}

impl ChargeKind {
    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tax => "TAX",
            Self::Shipping => "SHIPPING",
            Self::Fee => "FEE",
            Self::Discount => "DISCOUNT",
            Self::Other => "OTHER",
        }
    }
}

impl FromStr for ChargeKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "TAX" => Ok(Self::Tax),
            "SHIPPING" => Ok(Self::Shipping),
            "FEE" => Ok(Self::Fee),
            "DISCOUNT" => Ok(Self::Discount),
            "OTHER" => Ok(Self::Other),
            other => Err(format!("unknown charge kind `{other}`")),
        }
    }
}

/// Taxes, shipping, fees and discounts. `amount` is always positive; the kind
/// decides whether it adds to or subtracts from the total.
#[derive(Debug, Clone)]
pub struct QuotationCharge {
    pub kind: ChargeKind,
    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub description: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotationTotals {
    pub subtotal: Money,
    /// Net of discounts; may be negative.
    pub charges_total: Money,
    pub grand_total: Money,
}

#[derive(Debug, Clone)]
pub struct Quotation {
    pub id: QuotationId,
    pub quotation_number: String,
    pub customer_id: CustomerId,
    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub requirement_id: Option<Uuid>,
    pub currency: Currency,
    pub status: QuotationStatus,
    pub items: Vec<QuotationItem>,
    pub charges: Vec<QuotationCharge>,
    pub valid_until: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    /// Optimistic-concurrency version as loaded from the database.
    pub version: i32,
}

impl Quotation {
    /// Totals are always derived from lines and charges, never trusted from input.
    pub fn totals(&self) -> Result<QuotationTotals, QuotationError> {
        let line_totals = self
            .items
            .iter()
            .map(QuotationItem::line_total)
            .collect::<Result<Vec<_>, _>>()?;
        let subtotal = Money::sum(self.currency, line_totals)?;

        let charges_total =
            self.charges
                .iter()
                .try_fold(Money::zero(self.currency), |acc, charge| {
                    match charge.kind {
                        ChargeKind::Discount => acc.checked_sub(charge.amount),
                        _ => acc.checked_add(charge.amount),
                    }
                })?;

        let grand_total = subtotal.checked_add(charges_total)?;
        if grand_total.is_negative() {
            return Err(QuotationError::NegativeTotal);
        }

        Ok(QuotationTotals {
            subtotal,
            charges_total,
            grand_total,
        })
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used once SendQuotation is implemented")
    )]
    pub fn send(
        &mut self,
        now: DateTime<Utc>,
        valid_until: DateTime<Utc>,
    ) -> Result<(), QuotationError> {
        self.ensure_transition(QuotationStatus::Sent)?;
        if self.items.is_empty() {
            return Err(QuotationError::NoItems);
        }
        if valid_until <= now {
            return Err(QuotationError::InvalidValidity);
        }
        self.totals()?;

        self.status = QuotationStatus::Sent;
        self.sent_at = Some(now);
        self.valid_until = Some(valid_until);
        Ok(())
    }

    /// Business rule: a quotation can be accepted only while SENT and not past validity.
    pub fn accept(&mut self, now: DateTime<Utc>) -> Result<(), QuotationError> {
        self.ensure_transition(QuotationStatus::Accepted)?;
        if self.is_expired_at(now) {
            return Err(QuotationError::Expired);
        }
        self.status = QuotationStatus::Accepted;
        self.responded_at = Some(now);
        Ok(())
    }

    #[expect(dead_code, reason = "used once the TODO use case is implemented")]
    pub fn reject(
        &mut self,
        now: DateTime<Utc>,
        reason: Option<String>,
    ) -> Result<(), QuotationError> {
        self.ensure_transition(QuotationStatus::Rejected)?;
        self.status = QuotationStatus::Rejected;
        self.responded_at = Some(now);
        self.rejection_reason = reason;
        Ok(())
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        self.valid_until
            .is_some_and(|valid_until| valid_until <= now)
    }

    fn ensure_transition(&self, next: QuotationStatus) -> Result<(), QuotationError> {
        if self.status.can_transition_to(next) {
            Ok(())
        } else {
            Err(QuotationError::InvalidTransition {
                from: self.status,
                to: next,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn usd(value: &str) -> Money {
        Money::new(Decimal::from_str(value).unwrap(), Currency::Usd)
    }

    fn quotation(status: QuotationStatus) -> Quotation {
        Quotation {
            id: QuotationId::new(),
            quotation_number: "QT-2026-000001".into(),
            customer_id: CustomerId::new(),
            requirement_id: None,
            currency: Currency::Usd,
            status,
            items: vec![QuotationItem {
                line_number: 1,
                product_id: None,
                supplier_id: None,
                description: "Cotton fabric".into(),
                quantity: Decimal::from(1000),
                unit_of_measure: "m".into(),
                unit_price: usd("2.345"),
            }],
            charges: vec![
                QuotationCharge {
                    kind: ChargeKind::Shipping,
                    description: "Sea freight".into(),
                    amount: usd("150"),
                },
                QuotationCharge {
                    kind: ChargeKind::Discount,
                    description: "Volume".into(),
                    amount: usd("45"),
                },
            ],
            valid_until: None,
            sent_at: None,
            responded_at: None,
            rejection_reason: None,
            version: 1,
        }
    }

    #[test]
    fn totals_are_exact_and_apply_discounts() {
        let totals = quotation(QuotationStatus::Draft).totals().unwrap();
        assert_eq!(totals.subtotal, usd("2345.00"));
        assert_eq!(totals.charges_total, usd("105"));
        assert_eq!(totals.grand_total, usd("2450.00"));
    }

    #[test]
    fn cannot_accept_after_expiry() {
        let now = Utc::now();
        let mut quotation = quotation(QuotationStatus::Draft);
        quotation.send(now, now + Duration::days(7)).unwrap();

        let result = quotation.accept(now + Duration::days(8));
        assert!(matches!(result, Err(QuotationError::Expired)));
        assert_eq!(quotation.status, QuotationStatus::Sent);
    }

    #[test]
    fn accepts_sent_quotation_within_validity() {
        let now = Utc::now();
        let mut quotation = quotation(QuotationStatus::Draft);
        quotation.send(now, now + Duration::days(7)).unwrap();
        quotation.accept(now + Duration::days(1)).unwrap();
        assert_eq!(quotation.status, QuotationStatus::Accepted);
    }

    #[test]
    fn cannot_accept_draft() {
        let result = quotation(QuotationStatus::Draft).accept(Utc::now());
        assert!(matches!(
            result,
            Err(QuotationError::InvalidTransition { .. })
        ));
    }
}
