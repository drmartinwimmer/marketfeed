use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Controls whether alternative/black-market/digital currencies are included in OpenExchangeRates responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OxrAlternativeMode {
    /// Include alternative currencies (`show_alternative=true`).
    #[default]
    IncludeAlternative,
    /// Return only standard ISO currencies (`show_alternative=false`).
    StandardOnly,
}

impl OxrAlternativeMode {
    /// Returns the query string representation for `show_alternative`.
    #[must_use]
    pub const fn as_query_str(self) -> &'static str {
        match self {
            Self::IncludeAlternative => "true",
            Self::StandardOnly => "false",
        }
    }
}

/// Request parameters for querying OpenExchangeRates `/api/latest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OxrLatestRequest {
    /// Optional base currency code (defaults to `"USD"` on the server).
    pub base: Option<String>,
    /// Optional subset of target currency codes to return.
    pub symbols: Vec<String>,
    /// Controls whether alternative currencies are included.
    pub alternative: OxrAlternativeMode,
}

impl OxrLatestRequest {
    /// Creates a new [`OxrLatestRequest`] with default settings (including alternative currencies).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base currency code (e.g. `"EUR"`).
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = Some(base.into());
        self
    }

    /// Restricts the response to the given currency symbols.
    #[must_use]
    pub fn with_symbols<I, S>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = symbols.into_iter().map(Into::into).collect();
        self
    }

    /// Configures whether alternative currencies are included.
    #[must_use]
    pub const fn with_alternative(mut self, alternative: OxrAlternativeMode) -> Self {
        self.alternative = alternative;
        self
    }
}

/// Request parameters for querying OpenExchangeRates `/api/historical/{date}.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OxrHistoricalRequest {
    /// Historical date to query (`YYYY-MM-DD`).
    pub date: NaiveDate,
    /// Optional base currency code (defaults to `"USD"` on the server).
    pub base: Option<String>,
    /// Optional subset of target currency codes to return.
    pub symbols: Vec<String>,
    /// Controls whether alternative currencies are included.
    pub alternative: OxrAlternativeMode,
}

impl OxrHistoricalRequest {
    /// Creates a new [`OxrHistoricalRequest`] for the given `date`.
    #[must_use]
    pub const fn new(date: NaiveDate) -> Self {
        Self {
            date,
            base: None,
            symbols: Vec::new(),
            alternative: OxrAlternativeMode::IncludeAlternative,
        }
    }

    /// Sets the base currency code (e.g. `"EUR"`).
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = Some(base.into());
        self
    }

    /// Restricts the response to the given currency symbols.
    #[must_use]
    pub fn with_symbols<I, S>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = symbols.into_iter().map(Into::into).collect();
        self
    }

    /// Configures whether alternative currencies are included.
    #[must_use]
    pub const fn with_alternative(mut self, alternative: OxrAlternativeMode) -> Self {
        self.alternative = alternative;
        self
    }
}

/// Untransformed currency exchange rates response returned by OpenExchangeRates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct OxrRates {
    /// Unix epoch timestamp (in seconds) when the rates were published.
    pub timestamp: i64,
    /// Base currency code (typically `"USD"`).
    pub base: String,
    /// Map of currency code to exchange rate relative to `base`.
    pub rates: BTreeMap<String, Decimal>,
    /// Terms-of-use disclaimer URL.
    pub disclaimer: String,
    /// License agreement URL.
    pub license: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn oxr_rates_orders_chronologically_by_timestamp() {
        let earlier = OxrRates {
            timestamp: 1_700_000_000,
            base: "USD".to_string(),
            rates: BTreeMap::from([("EUR".to_string(), Decimal::new(92, 2))]),
            disclaimer: "z-disclaimer".to_string(),
            license: "z-license".to_string(),
        };
        let later = OxrRates {
            timestamp: 1_700_086_400,
            base: "USD".to_string(),
            rates: BTreeMap::from([("EUR".to_string(), Decimal::new(91, 2))]),
            disclaimer: "a-disclaimer".to_string(),
            license: "a-license".to_string(),
        };
        assert_that!(&earlier, lt(&later));
    }
}
