use crate::coinmarketcap::quote::CoinmarketcapStatus;
use serde::{Deserialize, Serialize};

/// Envelope returned by CoinMarketCap when an API request fails before returning `data`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct CoinmarketcapErrorEnvelope {
    pub(crate) status: CoinmarketcapStatus,
}

/// Errors that can occur when querying the CoinMarketCap Pro API.
#[derive(Debug, thiserror::Error)]
pub enum CoinmarketcapError {
    /// CoinMarketCap API key was not provided.
    #[error("missing CoinMarketCap API key")]
    MissingApiKey,

    /// Request did not specify any cryptocurrency symbols.
    #[error("at least one symbol is required for a CoinMarketCap quotes request")]
    MissingSymbols,

    /// Base or endpoint URL failed to parse.
    #[error("invalid CoinMarketCap URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// HTTP transport failure while communicating with CoinMarketCap.
    #[error("HTTP error communicating with CoinMarketCap: {0}")]
    Http(#[from] reqwest::Error),

    /// Response payload could not be deserialized as expected JSON.
    #[error("failed to parse CoinMarketCap JSON payload: {0}")]
    Json(#[from] serde_json::Error),

    /// CoinMarketCap returned a non-zero `status.error_code`.
    #[error("CoinMarketCap API error ({error_code}): {error_message}")]
    Server {
        /// Numeric CoinMarketCap error code.
        error_code: i64,
        /// Error description returned by CoinMarketCap.
        error_message: String,
    },

    /// CoinMarketCap returned a non-success HTTP status without a structured status envelope.
    #[error("CoinMarketCap returned unexpected HTTP status {status}: {body}")]
    UnexpectedStatus {
        /// HTTP status code.
        status: u16,
        /// Raw response body text.
        body: String,
    },
}
