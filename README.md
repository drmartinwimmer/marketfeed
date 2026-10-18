# marketfeed

[![CI](https://github.com/drmartinwimmer/marketfeed/actions/workflows/ci.yml/badge.svg)](https://github.com/drmartinwimmer/marketfeed/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/marketfeed.svg)](https://crates.io/crates/marketfeed)
[![Docs.rs](https://docs.rs/marketfeed/badge.svg)](https://docs.rs/marketfeed)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![MSRV: 1.88+](https://img.shields.io/badge/MSRV-1.88%2B-blue.svg?logo=rust)](https://www.rust-lang.org/)

Typed Rust client library and CLI for financial market and currency data sources.

## Overview

`marketfeed` provides a clean, stateless Rust interface and CLI binary for querying and parsing financial market data across multiple online APIs and file-based exchange exports. It returns strongly typed Rust structs directly from each backend without opinionated caching, storage, or domain transformations—while transparently handling multi-page pagination where applicable.

- **Direct Typed Backend Responses**: Deserializes backend responses directly into Rust structs using `rust_decimal::Decimal` and `chrono` timestamps.
- **Transparent Pagination**: Automatically handles offset/page iteration for paginated APIs (such as Marketstack) with configurable item, page, or full-fetch limits.
- **Stateless & Composable**: Pure data-source client library and CLI without local database, mirror, or updater coupling.

## Supported Data Sources

| Provider / Format     | Module                      | Description                                                                 |
| --------------------- | --------------------------- | --------------------------------------------------------------------------- |
| **Marketstack**       | `marketfeed::marketstack`   | End-of-day (`/v2/eod`) equity & ETF data with transparent pagination        |
| **CoinMarketCap**     | `marketfeed::coinmarketcap` | Latest cryptocurrency quotes (`/v2/cryptocurrency/quotes/latest`)           |
| **OpenExchangeRates** | `marketfeed::oxr`           | Latest (`/api/latest.json`) and historical (`/api/historical/*.json`) forex |
| **Xetra / Börse**     | `marketfeed::xetra`         | Deutsche Börse / Xetra semicolon-delimited historical CSV reader            |
| **Investing.com**     | `marketfeed::investing`     | Investing.com historical CSV reader (`K`/`M`/`B` volumes, `-` sentinels)    |

## Installation

### Library

```toml
[dependencies]
marketfeed = "0.1"
```

### CLI

```bash
cargo install marketfeed
```

## Library Usage

```rust,no_run
use marketfeed::marketstack::{MarketstackClient, MarketstackEodRequest};
use marketfeed::pagination::{PageLimit, SortOrder};
use marketfeed::xetra::XetraCsvReader;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Query all pages of Marketstack EOD data for AAPL and MSFT
    let client = MarketstackClient::new("YOUR_MARKETSTACK_ACCESS_KEY")?;
    let request = MarketstackEodRequest::new(["AAPL", "MSFT"])
        .with_limit(PageLimit::All)
        .with_sort(SortOrder::Asc);
    let entries = client.eod(&request)?;
    println!("Fetched {} EOD entries", entries.len());

    // Parse a local Xetra CSV export
    let xetra_entries = XetraCsvReader::read_path(Path::new("ENR.csv"))?;
    println!("Parsed {} Xetra rows", xetra_entries.len());

    Ok(())
}
```

## CLI & Secrets Configuration

`marketfeed` loads API credentials (`MarketfeedSecrets`) from `<config_dir>/marketfeed/secrets.json` in the user's home configuration directory (e.g. `~/.config/marketfeed/secrets.json` via `MarketfeedSecrets::from_default_path()`), or from a custom path in Rust via `MarketfeedSecrets::from_path(path)`:

```json
{
  "marketstack_access_key": "YOUR_MARKETSTACK_ACCESS_KEY",
  "coinmarketcap_api_key": "YOUR_COINMARKETCAP_API_KEY",
  "oxr_app_id": "YOUR_OXR_APP_ID"
}
```

```bash
# Fetch all EOD pages for AAPL and MSFT since 2025-01-01
marketfeed marketstack eod --symbols AAPL,MSFT --date-from 2025-01-01 --all-pages

# Fetch latest BTC and ETH quotes in USD and EUR
marketfeed coinmarketcap quotes --symbols BTC,ETH --convert USD,EUR

# Fetch historical OpenExchangeRates for a specific date
marketfeed oxr historical --date 2025-04-04

# Parse a local Xetra or Investing.com CSV export to JSON
marketfeed xetra parse --file ENR.csv
marketfeed investing parse --file GOOG.csv
```

## License

MIT
