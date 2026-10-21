use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error payload returned by the Marketstack REST API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MarketstackServerError {
    /// Machine-readable Marketstack error code (e.g. `"invalid_access_key"`).
    pub code: String,
    /// Human-readable error message returned by Marketstack.
    pub message: String,
    /// Optional structured context object detailing invalid parameters.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

impl fmt::Display for MarketstackServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Envelope wrapping a [`MarketstackServerError`] in HTTP error responses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct MarketstackErrorEnvelope {
    pub(crate) error: MarketstackServerError,
}

/// Errors that can occur when querying the Marketstack API.
#[derive(Debug, thiserror::Error)]
pub enum MarketstackError {
    /// Marketstack access key was not provided.
    #[error("missing Marketstack access key")]
    MissingAccessKey,

    /// Request did not specify any ticker symbols.
    #[error("at least one symbol is required for a Marketstack EOD request")]
    MissingSymbols,

    /// Requested page item limit exceeds the Marketstack per-page maximum.
    #[error("invalid Marketstack page limit {limit} (maximum is {max})")]
    InvalidPaginationLimit {
        /// Requested item count per page.
        limit: u16,
        /// Maximum supported item count per page.
        max: u16,
    },

    /// Base or endpoint URL failed to parse.
    #[error("invalid Marketstack URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// HTTP transport failure while communicating with Marketstack.
    #[error("HTTP error communicating with Marketstack: {0}")]
    Http(#[from] reqwest::Error),

    /// Response payload could not be deserialized as expected JSON.
    #[error("failed to parse Marketstack JSON payload: {0}")]
    Json(#[from] serde_json::Error),

    /// Marketstack returned a structured API error payload.
    #[error("Marketstack API error ({0})")]
    Server(MarketstackServerError),

    /// Marketstack returned a non-success HTTP status without a structured error payload.
    #[error("Marketstack returned unexpected HTTP status {status}: {body}")]
    UnexpectedStatus {
        /// HTTP status code.
        status: u16,
        /// Raw response body text.
        body: String,
    },
}
