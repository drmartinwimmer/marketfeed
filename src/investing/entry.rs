use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Single historical trading session entry parsed from an Investing.com CSV export.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct InvestingEntry {
    /// Trading session date (`"Date"`, formatted `%m/%d/%Y`).
    pub date: NaiveDate,
    /// Closing / latest price (`"Price"`).
    pub price: Option<Decimal>,
    /// Opening price (`"Open"`).
    pub open: Option<Decimal>,
    /// Session high price (`"High"`).
    pub high: Option<Decimal>,
    /// Session low price (`"Low"`).
    pub low: Option<Decimal>,
    /// Trading volume (`"Vol."`), expanded from `K` (`10^3`), `M` (`10^6`), or `B` (`10^9`) suffixes.
    pub volume: Option<Decimal>,
    /// Daily percentage change (`"Change %"`), with the trailing `%` sign removed.
    pub change_percent: Option<Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn investing_entry_orders_chronologically_by_date()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 3).ok_or("invalid date")?;
        let d2 = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        let e1 = InvestingEntry {
            date: d1,
            price: Some(Decimal::from(150)),
            open: None,
            high: None,
            low: None,
            volume: None,
            change_percent: None,
        };
        let e2 = InvestingEntry {
            date: d2,
            ..e1.clone()
        };
        assert_that!(&e1, lt(&e2));
        Ok(())
    }
}
