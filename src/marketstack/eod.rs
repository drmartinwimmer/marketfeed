use crate::pagination::{PageLimit, PaginationMeta, SortOrder};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Request parameters for querying Marketstack's `/v2/eod` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketstackEodRequest {
    /// One or more ticker symbols to query (e.g. `["AAPL", "MSFT"]`).
    pub symbols: Vec<String>,
    /// Optional pagination limit (`Items`, `Pages`, or `All`).
    pub limit: Option<PageLimit>,
    /// Optional chronological sort order (`Asc` or `Desc`).
    pub sort: Option<SortOrder>,
    /// Optional inclusive start date (`YYYY-MM-DD`).
    pub date_from: Option<NaiveDate>,
    /// Optional inclusive end date (`YYYY-MM-DD`).
    pub date_to: Option<NaiveDate>,
    /// Optional exchange MIC or identifier filter (e.g. `"XNAS"`).
    pub exchange: Option<String>,
}

impl MarketstackEodRequest {
    /// Creates a new [`MarketstackEodRequest`] for the given ticker symbols.
    #[must_use]
    pub fn new<I, S>(symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            limit: None,
            sort: None,
            date_from: None,
            date_to: None,
            exchange: None,
        }
    }

    /// Sets the pagination limit strategy.
    #[must_use]
    pub const fn with_limit(mut self, limit: PageLimit) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the sort order (`Asc` or `Desc`).
    #[must_use]
    pub const fn with_sort(mut self, sort: SortOrder) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Sets the inclusive start date (`date_from`).
    #[must_use]
    pub const fn with_date_from(mut self, date_from: NaiveDate) -> Self {
        self.date_from = Some(date_from);
        self
    }

    /// Sets the inclusive end date (`date_to`).
    #[must_use]
    pub const fn with_date_to(mut self, date_to: NaiveDate) -> Self {
        self.date_to = Some(date_to);
        self
    }

    /// Sets the exchange filter (e.g. `"XNAS"`).
    #[must_use]
    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = Some(exchange.into());
        self
    }
}

/// Serde adapter for Marketstack timestamps (`%Y-%m-%dT%H:%M:%S%z` or RFC 3339).
struct MarketstackTimestampFormat;

impl MarketstackTimestampFormat {
    fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if let Ok(dt) = DateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M:%S%z") {
            return Ok(dt.with_timezone(&Utc));
        }
        DateTime::parse_from_rfc3339(&raw)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(DeError::custom)
    }

    fn serialize<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_rfc3339())
    }
}

/// Single end-of-day (EOD) market data record returned by Marketstack `/v2/eod`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MarketstackEodEntry {
    /// Trading session timestamp in UTC.
    #[serde(
        deserialize_with = "MarketstackTimestampFormat::deserialize",
        serialize_with = "MarketstackTimestampFormat::serialize"
    )]
    pub date: DateTime<Utc>,
    /// Unadjusted opening price.
    pub open: Decimal,
    /// Split- and dividend-adjusted opening price.
    #[serde(default)]
    pub adj_open: Option<Decimal>,
    /// Unadjusted closing price.
    pub close: Decimal,
    /// Split- and dividend-adjusted closing price.
    #[serde(default)]
    pub adj_close: Option<Decimal>,
    /// Unadjusted session high price.
    pub high: Decimal,
    /// Split- and dividend-adjusted session high price.
    #[serde(default)]
    pub adj_high: Option<Decimal>,
    /// Unadjusted session low price.
    pub low: Decimal,
    /// Split- and dividend-adjusted session low price.
    #[serde(default)]
    pub adj_low: Option<Decimal>,
    /// Unadjusted trading volume.
    pub volume: Decimal,
    /// Split-adjusted trading volume.
    #[serde(default)]
    pub adj_volume: Option<Decimal>,
    /// Split factor applied on this session date, if provided.
    #[serde(default)]
    pub split_factor: Option<Decimal>,
    /// Cash dividend distributed on this session date, if provided.
    #[serde(default)]
    pub dividend: Option<Decimal>,
    /// Security or company display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Ticker symbol (e.g. `"AAPL"`).
    pub symbol: String,
    /// Market identifier code (MIC) of the exchange (e.g. `"XNAS"`).
    pub exchange: String,
    /// Short exchange code (e.g. `"NASDAQ"`).
    #[serde(default)]
    pub exchange_code: Option<String>,
    /// Security asset class (e.g. `"Stock"`).
    #[serde(default)]
    pub asset_type: Option<String>,
    /// ISO currency code in which prices are quoted (e.g. `"USD"`).
    #[serde(default)]
    pub price_currency: Option<String>,
}

/// Paginated response envelope returned by Marketstack `/v2/eod`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MarketstackEodResponse {
    /// Pagination metadata for the returned page.
    pub pagination: PaginationMeta,
    /// EOD entries contained in the current page.
    pub data: Vec<MarketstackEodEntry>,
}
