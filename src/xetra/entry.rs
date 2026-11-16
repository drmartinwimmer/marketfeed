use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Single historical trading session entry parsed from a Deutsche Börse / Xetra CSV export.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct XetraEntry {
    /// Trading session date.
    pub date: NaiveDate,
    /// Opening price (`Opening price` / `Open`).
    pub open: Decimal,
    /// Closing price (`Closing price` / `Close`).
    pub close: Decimal,
    /// Session high price (`Daily high` / `High`).
    pub high: Decimal,
    /// Session low price (`Daily low` / `Low`).
    pub low: Decimal,
    /// Total traded turnover in currency units (`Turnover` / `Volume`).
    pub volume: u64,
    /// Total traded quantity in shares/units (`Traded volume` / `Volume units`).
    pub volume_units: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn xetra_entry_orders_chronologically_by_date()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 3).ok_or("invalid date")?;
        let d2 = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        let e1 = XetraEntry {
            date: d1,
            open: Decimal::from(100),
            close: Decimal::from(101),
            high: Decimal::from(102),
            low: Decimal::from(99),
            volume: 1_000,
            volume_units: 10,
        };
        let e2 = XetraEntry {
            date: d2,
            ..e1.clone()
        };
        assert_that!(&e1, lt(&e2));
        Ok(())
    }
}
