//! OpenExchangeRates API client and exchange rate data types.

mod client;
mod error;
mod rates;

pub use client::OxrClient;
pub use error::{OxrError, OxrServerError};
pub use rates::{OxrAlternativeMode, OxrHistoricalRequest, OxrLatestRequest, OxrRates};
