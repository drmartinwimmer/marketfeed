use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Single historical trading session entry parsed from an Investing.com CSV export.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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
