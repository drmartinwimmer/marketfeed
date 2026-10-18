use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Single historical trading session entry parsed from a Deutsche Börse / Xetra CSV export.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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
