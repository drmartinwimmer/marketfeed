//! Marketstack REST API v2 client and end-of-day (EOD) data types.

mod client;
mod eod;
mod error;

pub use client::MarketstackClient;
pub use eod::{MarketstackEodEntry, MarketstackEodRequest, MarketstackEodResponse};
pub use error::{MarketstackError, MarketstackServerError};
