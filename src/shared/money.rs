//! Monetary values. Never use floating point for money.
//!
//! Database columns: `NUMERIC(19,4)` for amounts plus a `CHAR(3)` currency code
//! referencing `currencies(code)`. JSON: amounts are serialised as strings
//! (`"1250.50"`) so JavaScript clients cannot lose precision.

use std::{fmt, str::FromStr};

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Lkr,
    Usd,
    Inr,
    Cny,
    Aed,
    Eur,
}

impl Currency {
    pub fn code(self) -> &'static str {
        match self {
            Self::Lkr => "LKR",
            Self::Usd => "USD",
            Self::Inr => "INR",
            Self::Cny => "CNY",
            Self::Aed => "AED",
            Self::Eur => "EUR",
        }
    }

    /// ISO 4217 minor units (decimal places used for settlement amounts).
    pub fn minor_units(self) -> u32 {
        2
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            "LKR" => Ok(Self::Lkr),
            "USD" => Ok(Self::Usd),
            "INR" => Ok(Self::Inr),
            "CNY" => Ok(Self::Cny),
            "AED" => Ok(Self::Aed),
            "EUR" => Ok(Self::Eur),
            _ => Err(MoneyError::UnsupportedCurrency(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("unsupported currency `{0}`")]
    UnsupportedCurrency(String),

    #[error("currency mismatch: {left} vs {right}")]
    CurrencyMismatch { left: Currency, right: Currency },

    #[error("monetary arithmetic overflow")]
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Money {
    #[serde(with = "rust_decimal::serde::str")]
    #[schema(value_type = String, example = "1250.50")]
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub fn zero(currency: Currency) -> Self {
        Self::new(Decimal::ZERO, currency)
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative() && !self.amount.is_zero()
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_same_currency(other)?;
        let amount = self
            .amount
            .checked_add(other.amount)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self::new(amount, self.currency))
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_same_currency(other)?;
        let amount = self
            .amount
            .checked_sub(other.amount)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self::new(amount, self.currency))
    }

    /// Unit price × quantity (quantities may be fractional, e.g. kilograms).
    pub fn checked_mul(self, factor: Decimal) -> Result<Self, MoneyError> {
        let amount = self
            .amount
            .checked_mul(factor)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self::new(amount, self.currency))
    }

    /// Rounds to the currency's minor units (half away from zero, commercial rounding).
    pub fn round_to_minor_units(self) -> Self {
        let amount = self.amount.round_dp_with_strategy(
            self.currency.minor_units(),
            RoundingStrategy::MidpointAwayFromZero,
        );
        Self::new(amount, self.currency)
    }

    pub fn sum<I>(currency: Currency, values: I) -> Result<Self, MoneyError>
    where
        I: IntoIterator<Item = Self>,
    {
        values
            .into_iter()
            .try_fold(Self::zero(currency), Self::checked_add)
    }

    fn ensure_same_currency(self, other: Self) -> Result<(), MoneyError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn usd(value: &str) -> Money {
        Money::new(Decimal::from_str(value).unwrap(), Currency::Usd)
    }

    #[test]
    fn adds_same_currency_exactly() {
        assert_eq!(usd("0.10").checked_add(usd("0.20")).unwrap(), usd("0.30"));
    }

    #[test]
    fn rejects_currency_mismatch() {
        let lkr = Money::new(Decimal::ONE, Currency::Lkr);
        assert!(matches!(
            usd("1").checked_add(lkr),
            Err(MoneyError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(usd("10.005").round_to_minor_units(), usd("10.01"));
    }

    #[test]
    fn serialises_amount_as_string() {
        let json = serde_json::to_value(usd("1250.50")).unwrap();
        assert_eq!(json["amount"], "1250.50");
        assert_eq!(json["currency"], "USD");
    }
}
