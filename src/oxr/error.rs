use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error payload returned by the OpenExchangeRates API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OxrServerError {
    /// HTTP status code reported in the JSON payload.
    pub status: u16,
    /// Short machine-readable error token (e.g. `"invalid_app_id"`).
    pub message: String,
    /// Detailed human-readable error description.
    pub description: String,
}

impl fmt::Display for OxrServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.message, self.status, self.description
        )
    }
}

/// Errors that can occur when querying the OpenExchangeRates API.
#[derive(Debug, thiserror::Error)]
pub enum OxrError {
    /// OpenExchangeRates App ID was not provided.
    #[error("missing OpenExchangeRates App ID")]
    MissingAppId,

    /// Base or endpoint URL failed to parse.
    #[error("invalid OpenExchangeRates URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// HTTP transport failure while communicating with OpenExchangeRates.
    #[error("HTTP error communicating with OpenExchangeRates: {0}")]
    Http(#[from] reqwest::Error),

    /// Response payload could not be deserialized as expected JSON.
    #[error("failed to parse OpenExchangeRates JSON payload: {0}")]
    Json(#[from] serde_json::Error),

    /// OpenExchangeRates returned a structured API error payload.
    #[error("OpenExchangeRates API error ({0})")]
    Server(OxrServerError),

    /// OpenExchangeRates returned a non-success HTTP status without a structured error payload.
    #[error("OpenExchangeRates returned unexpected HTTP status {status}: {body}")]
    UnexpectedStatus {
        /// HTTP status code.
        status: u16,
        /// Raw response body text.
        body: String,
    },
}
