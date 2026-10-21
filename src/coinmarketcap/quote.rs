use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request parameters for querying CoinMarketCap's `/v2/cryptocurrency/quotes/latest` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoinmarketcapQuotesRequest {
    /// Cryptocurrency symbols to query (e.g. `["BTC", "ETH"]`).
    pub symbols: Vec<String>,
    /// Optional target quote currencies (e.g. `["USD", "EUR"]`).
    pub convert: Vec<String>,
}

impl CoinmarketcapQuotesRequest {
    /// Creates a new [`CoinmarketcapQuotesRequest`] for the given cryptocurrency symbols.
    #[must_use]
    pub fn new<I, S>(symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            convert: Vec::new(),
        }
    }

    /// Specifies target conversion currencies (e.g. `["USD", "EUR"]`).
    #[must_use]
    pub fn with_convert<I, S>(mut self, convert: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.convert = convert.into_iter().map(Into::into).collect();
        self
    }
}

/// Response status metadata returned by the CoinMarketCap Pro API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapStatus {
    /// Server timestamp when the response was generated.
    pub timestamp: DateTime<Utc>,
    /// API error code (`0` indicates success).
    pub error_code: i64,
    /// Optional human-readable error message when `error_code != 0`.
    #[serde(default)]
    pub error_message: Option<String>,
    /// Server processing time in milliseconds.
    pub elapsed: i64,
    /// API credits consumed by the call.
    pub credit_count: i64,
    /// Optional notice message from CoinMarketCap.
    #[serde(default)]
    pub notice: Option<String>,
}

/// Tag metadata entry attached to a CoinMarketCap asset quote in `/v2/cryptocurrency/quotes/latest`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapTag {
    /// URL-friendly tag slug.
    pub slug: String,
    /// Human-readable tag name.
    pub name: String,
    /// Tag category (e.g. `"ALGORITHM"`, `"CATEGORY"`, `"INDUSTRY"`).
    pub category: String,
}

/// Parent blockchain platform metadata for tokenized assets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapPlatform {
    /// CoinMarketCap ID of the host blockchain platform.
    pub id: u64,
    /// Display name of the host platform (e.g. `"Ethereum"`).
    pub name: String,
    /// Ticker symbol of the host platform (e.g. `"ETH"`).
    pub symbol: String,
    /// URL slug of the host platform.
    pub slug: String,
    /// Contract or token address on the host platform.
    pub token_address: String,
}

/// Market quote metrics in a specific target currency (e.g. `"USD"` or `"EUR"`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapCurrencyQuote {
    /// Latest unit price in the target currency (`None` if unavailable/untracked).
    #[serde(default)]
    pub price: Option<Decimal>,
    /// Rolling 24-hour trading volume in the target currency.
    #[serde(default)]
    pub volume_24h: Option<Decimal>,
    /// 24-hour percentage change in trading volume.
    #[serde(default)]
    pub volume_change_24h: Option<Decimal>,
    /// 1-hour percentage price change.
    #[serde(default)]
    pub percent_change_1h: Option<Decimal>,
    /// 24-hour percentage price change.
    #[serde(default)]
    pub percent_change_24h: Option<Decimal>,
    /// 7-day percentage price change.
    #[serde(default)]
    pub percent_change_7d: Option<Decimal>,
    /// 30-day percentage price change.
    #[serde(default)]
    pub percent_change_30d: Option<Decimal>,
    /// 60-day percentage price change.
    #[serde(default)]
    pub percent_change_60d: Option<Decimal>,
    /// 90-day percentage price change.
    #[serde(default)]
    pub percent_change_90d: Option<Decimal>,
    /// Market capitalization in the target currency.
    #[serde(default)]
    pub market_cap: Option<Decimal>,
    /// Market capitalization dominance percentage.
    #[serde(default)]
    pub market_cap_dominance: Option<Decimal>,
    /// Fully diluted market capitalization in the target currency.
    #[serde(default)]
    pub fully_diluted_market_cap: Option<Decimal>,
    /// Total value locked (TVL) in the target currency, if applicable.
    #[serde(default)]
    pub tvl: Option<Decimal>,
    /// Timestamp when this currency quote was last updated.
    #[serde(default)]
    pub last_updated: Option<DateTime<Utc>>,
}

/// Untransformed cryptocurrency asset record returned by CoinMarketCap `/v2/cryptocurrency/quotes/latest`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapAssetQuote {
    /// Unique CoinMarketCap asset identifier.
    pub id: u64,
    /// Asset display name (e.g. `"Bitcoin"`).
    pub name: String,
    /// Asset ticker symbol (e.g. `"BTC"`).
    pub symbol: String,
    /// Asset URL slug (e.g. `"bitcoin"`).
    pub slug: String,
    /// Number of active market pairs tracked by CoinMarketCap.
    #[serde(default)]
    pub num_market_pairs: Option<u64>,
    /// Timestamp when the asset was added to CoinMarketCap.
    #[serde(default)]
    pub date_added: Option<DateTime<Utc>>,
    /// Structured tags associated with the asset.
    #[serde(default)]
    pub tags: Vec<CoinmarketcapTag>,
    /// Maximum supply cap, if bounded.
    #[serde(default)]
    pub max_supply: Option<Decimal>,
    /// Currently circulating supply.
    #[serde(default)]
    pub circulating_supply: Option<Decimal>,
    /// Total existing supply.
    #[serde(default)]
    pub total_supply: Option<Decimal>,
    /// Active status indicator (`1` for active, `0` for inactive).
    #[serde(default)]
    pub is_active: Option<u8>,
    /// Whether the asset has an uncapped/infinite supply.
    #[serde(default)]
    pub infinite_supply: Option<bool>,
    /// Minted market capitalization, if reported.
    #[serde(default)]
    pub minted_market_cap: Option<Decimal>,
    /// Parent blockchain platform metadata for tokens.
    #[serde(default)]
    pub platform: Option<CoinmarketcapPlatform>,
    /// CoinMarketCap rank by market capitalization.
    #[serde(default)]
    pub cmc_rank: Option<u64>,
    /// Whether the asset represents a fiat currency (`1` for fiat, `0` for crypto).
    #[serde(default)]
    pub is_fiat: Option<u8>,
    /// Project self-reported circulating supply.
    #[serde(default)]
    pub self_reported_circulating_supply: Option<Decimal>,
    /// Project self-reported market capitalization.
    #[serde(default)]
    pub self_reported_market_cap: Option<Decimal>,
    /// Total value locked (TVL) ratio, if applicable.
    #[serde(default)]
    pub tvl_ratio: Option<Decimal>,
    /// Timestamp when the asset metadata was last updated.
    #[serde(default)]
    pub last_updated: Option<DateTime<Utc>>,
    /// Map of currency symbol (e.g. `"USD"`, `"EUR"`) to quote metrics.
    pub quote: HashMap<String, CoinmarketcapCurrencyQuote>,
}

/// Complete response envelope returned by CoinMarketCap `/v2/cryptocurrency/quotes/latest`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinmarketcapQuotesResponse {
    /// API response status metadata.
    pub status: CoinmarketcapStatus,
    /// Map from uppercase ticker symbol to all matching CoinMarketCap assets.
    pub data: HashMap<String, Vec<CoinmarketcapAssetQuote>>,
}
