//! CoinMarketCap Pro API v2 client and cryptocurrency quote data types.

mod client;
mod error;
mod quote;

pub use client::CoinmarketcapClient;
pub use error::CoinmarketcapError;
pub use quote::{
    CoinmarketcapAssetQuote, CoinmarketcapCurrencyQuote, CoinmarketcapPlatform,
    CoinmarketcapQuotesRequest, CoinmarketcapQuotesResponse, CoinmarketcapStatus, CoinmarketcapTag,
};
