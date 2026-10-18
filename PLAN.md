# `marketfeed` Implementation Plan

## 1. Vision & Design Goals

`marketfeed` extracts and generalizes the financial market data-source interfaces from `stock` into a standalone, reusable Rust library crate and companion CLI binary.

### Core Principles

1. **Pure Data-Source Interfaces**: Only the client/reader interfaces to external market data backends belong in `marketfeed`. Local caching (`*Mirror`), sharded file persistence (`ShardKey`, `Shardable`), incremental state tracking (`*Updater`), symbol interning (`SymbolTable`), and series combination/adjustment (`Ticker`) remain in downstream consumers like `stock`.
2. **Untransformed Typed Rust Structs**: Backend payloads are deserialized directly into strongly typed Rust structs (using `rust_decimal::Decimal` for monetary/volume figures and `chrono` for dates/timestamps) and returned without lossy domain transformations:
   - No `Decimal` to `f32` conversions.
   - No filtering out records or flattening multi-currency maps (e.g., CoinMarketCap's `quote` map preserves all returned quote currencies rather than hardcoding `"USD"` and dropping null-priced entries).
   - No injecting synthetic domain fields into parsed records unless part of the source payload.
3. **Transparent Pagination**: Multi-page HTTP endpoints (such as Marketstack's `offset`/`limit`/`total` pagination) hide their paging loop behind a declarative `Limit` specification (`Items(NonZero<u16>)`, `Pages(NonZero<u8>)`, or `All` with a configurable safety ceiling) and return a unified `Vec<Entry>`.
4. **Strict Code Hygiene (`purist` & `clippy`)**: Every module, struct, error enum, and test is designed from the ground up to satisfy `purist` AST rules (`<Subject>Error`, `<Subject>Command`, `<Subject>Config`, no free functions, no primitive boolean flag clutter, no `println!` in library code, no env access outside config) and workspace Clippy lints (`unwrap_used = "warn"`, `indexing_slicing = "warn"`, etc.).

---

## 2. Module & Provider Architecture

```text
src/
├── lib.rs
├── main.rs
├── commands.rs          # CLI command definitions and execution (MarketfeedCommand)
├── config.rs            # CLI / environment credential configuration (MarketfeedConfig)
├── pagination.rs        # Shared pagination types (Limit, PaginationMeta, Sort)
├── marketstack/         # Marketstack REST API v2 client & types
│   ├── mod.rs
│   ├── client.rs
│   ├── eod.rs
│   └── error.rs
├── coinmarketcap/       # CoinMarketCap Pro API v2 client & types
│   ├── mod.rs
│   ├── client.rs
│   ├── quote.rs
│   └── error.rs
├── oxr/                 # OpenExchangeRates API client & types
│   ├── mod.rs
│   ├── client.rs
│   ├── rates.rs
│   └── error.rs
├── xetra/               # Deutsche Börse / Xetra CSV reader & types
│   ├── mod.rs
│   ├── reader.rs
│   ├── entry.rs
│   └── error.rs
└── investing/           # Investing.com historical CSV reader & types
    ├── mod.rs
    ├── reader.rs
    ├── entry.rs
    └── error.rs
```

### 2.1 Shared Pagination (`marketfeed::pagination`)

Provides reusable request pagination controls and response metadata:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLimit {
    /// Fetch a single page with at most `N` items.
    Items(NonZero<u16>),
    /// Fetch up to `N` pages and concatenate all returned items.
    Pages(NonZero<u8>),
    /// Fetch all available pages up to the endpoint's safety ceiling.
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct PaginationMeta {
    pub limit: u32,
    pub offset: u32,
    pub count: u32,
    pub total: u32,
}
```

### 2.2 Marketstack Provider (`marketfeed::marketstack`)

Generalizes `stock::marketstack::endpoint` and `stock::marketstack::eod`:

- **`MarketstackClient`**:
  - `MarketstackClient::new(access_key: String) -> Result<Self, MarketstackError>`
  - `MarketstackClient::with_base_url(access_key: String, base_url: Url) -> Self` (enables deterministic `mockito` testing in both `marketfeed` and downstream crates).
  - `eod(&self, request: &MarketstackEodRequest) -> Result<Vec<MarketstackEodEntry>, MarketstackError>`: Transparently handles single-page or multi-page (`offset` += `pagination.count` while `offset < pagination.total`) requests according to `request.limit`.
- **`MarketstackEodRequest`**:
  - Fields: `symbols: Vec<String>`, `limit: Option<PageLimit>`, `sort: Option<SortOrder>`, `date_from: Option<NaiveDate>`, `date_to: Option<NaiveDate>`, `exchange: Option<String>`.
- **`MarketstackEodEntry`**:
  - Directly mirrors the `/v2/eod` JSON object (`date: DateTime<Utc>`, `open`, `adj_open`, `close`, `adj_close`, `high`, `adj_high`, `low`, `adj_low`, `volume`, `adj_volume`, `split_factor`, `dividend`, `name`, `symbol`, `exchange`, `exchange_code`, `asset_type`, `price_currency`).
- **`MarketstackError` & `MarketstackServerError`**:
  - Structured error handling for HTTP transport (`reqwest::Error`), URL construction (`url::ParseError`), invalid pagination limits, and structured API error payloads (`code`, `message`, `context`).

### 2.3 CoinMarketCap Provider (`marketfeed::coinmarketcap`)

Generalizes `stock::coinmarketcap::endpoint` and `stock::coinmarketcap::quote`:

- **`CoinmarketcapClient`**:
  - `CoinmarketcapClient::new(api_key: String) -> Result<Self, CoinmarketcapError>`
  - `CoinmarketcapClient::with_base_url(api_key: String, base_url: Url) -> Self`
  - `quotes_latest(&self, request: &CoinmarketcapQuotesRequest) -> Result<CoinmarketcapQuotesResponse, CoinmarketcapError>`
- **`CoinmarketcapQuotesRequest`**:
  - Supports querying by `symbols: Vec<String>` (with optional `convert: Vec<String>` currencies) rather than hardcoding USD-only lookups.
- **Untransformed Response Structs**:
  - Unlike `stock`, which flattened `QuoteData` + `QuoteUsd` into `CoinmarketcapQuoteEntry` and silently dropped entries with `price: null`, `marketfeed` returns the full `CoinmarketcapQuotesResponse`:
    - `status: CoinmarketcapStatus` (`timestamp`, `error_code`, `error_message`, `elapsed`, `credit_count`)
    - `data: HashMap<String, Vec<CoinmarketcapAssetQuote>>` containing all asset metadata (`id`, `name`, `symbol`, `slug`, `num_market_pairs`, `date_added`, `tags`, `max_supply`, `circulating_supply`, `total_supply`, `is_active`, `infinite_supply`, `minted_market_cap`, `platform`, `cmc_rank`, `is_fiat`, `self_reported_circulating_supply`, `self_reported_market_cap`, `tvl_ratio`, `last_updated`) and `quote: HashMap<String, CoinmarketcapCurrencyQuote>`.

### 2.4 OpenExchangeRates Provider (`marketfeed::oxr`)

Generalizes `stock::oxr::endpoint` and `stock::oxr::data`:

- **`OxrClient`**:
  - `OxrClient::new(app_id: String) -> Result<Self, OxrError>`
  - `OxrClient::with_base_url(app_id: String, base_url: Url) -> Self`
  - `latest(&self, request: &OxrLatestRequest) -> Result<OxrRates, OxrError>`
  - `historical(&self, request: &OxrHistoricalRequest) -> Result<OxrRates, OxrError>`
- **`OxrLatestRequest` & `OxrHistoricalRequest`**:
  - Allows configuring `base: Option<String>`, `symbols: Vec<String>`, and `alternative: OxrAlternativeMode` (defaulting to including alternative currencies as in `stock`).
- **`OxrRates`**:
  - Raw response struct (`disclaimer: String`, `license: String`, `timestamp: i64`, `base: String`, `rates: HashMap<String, Decimal>`).
  - Excludes `stock`-specific `TickerPoint` and `f32` cross-rate conversion methods.

### 2.5 Xetra CSV Reader (`marketfeed::xetra`)

Generalizes `stock::import::xetra`:

- **`XetraCsvReader`**:
  - `XetraCsvReader::read_path(path: &Path) -> Result<Vec<XetraEntry>, XetraError>`
  - `XetraCsvReader::read_str(content: &str) -> Result<Vec<XetraEntry>, XetraError>`
  - `XetraCsvReader::read<R: BufRead>(reader: R) -> Result<Vec<XetraEntry>, XetraError>`
  - Automatically skips Xetra's preamble header line (`Xetra;<ISIN>;"";"";"";"";""`) and deserializes semicolon-delimited rows.
- **`XetraEntry`**:
  - `date: NaiveDate` (`%d/%m/%y`), `open: Decimal`, `close: Decimal`, `high: Decimal`, `low: Decimal`, `volume: u64`, `volume_units: u64`.

### 2.6 Investing.com CSV Reader (`marketfeed::investing`)

Generalizes `stock::investing::csv`:

- **`InvestingCsvReader`**:
  - `InvestingCsvReader::read_path(path: &Path) -> Result<Vec<InvestingEntry>, InvestingError>`
  - `InvestingCsvReader::read_str(content: &str) -> Result<Vec<InvestingEntry>, InvestingError>`
  - `InvestingCsvReader::read<R: Read>(reader: R) -> Result<Vec<InvestingEntry>, InvestingError>`
- **`InvestingEntry`**:
  - Pure representation of the CSV row without requiring a caller-injected `symbol` string or sorting side effects:
    - `date: NaiveDate` (`%m/%d/%Y`)
    - `price: Option<Decimal>` (mapped from `"Price"`)
    - `open: Option<Decimal>`
    - `high: Option<Decimal>`
    - `low: Option<Decimal>`
    - `volume: Option<Decimal>` (parsed from `"Vol."` supporting `K`, `M`, `B` multipliers and `-` empty markers)
    - `change_percent: Option<Decimal>` (or raw string/decimal from `"Change %"`)

---

## 3. CLI Binary Design (`marketfeed`)

The `marketfeed` binary exposes each provider as a subcommand and outputs JSON to `stdout`:

```text
marketfeed <SUBCOMMAND>

Subcommands:
  marketstack     Query Marketstack end-of-day (EOD) market data
  coinmarketcap   Query CoinMarketCap cryptocurrency quotes
  oxr             Query OpenExchangeRates latest or historical currency rates
  xetra           Parse Deutsche Börse / Xetra historical CSV files
  investing       Parse Investing.com historical CSV files
```

### Example Invocations

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

API keys can be supplied via `--api-key` / `--access-key` / `--app-id` CLI flags or read from environment variables (`MARKETSTACK_ACCESS_KEY`, `COINMARKETCAP_API_KEY`, `OXR_APP_ID`) inside `src/config.rs`.

---

## 4. Mapping from `stock` to `marketfeed`

| `stock` Module / Type                                            | `marketfeed` Equivalent                                                 | Disposition of Remaining `stock` Code                                                     |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `stock::marketstack::MarketstackEndpoint`                        | `marketfeed::marketstack::MarketstackClient`                            | `MarketstackMirror`, `MarketstackUpdater`, and `Shardable` impls stay in `stock`          |
| `stock::marketstack::EodRequest` & `MarketstackEodEntry`         | `marketfeed::marketstack::{MarketstackEodRequest, MarketstackEodEntry}` | Imported directly by `stock`                                                              |
| `stock::marketstack::endpoint::{Limit, Pagination, Sort}`        | `marketfeed::pagination::{PageLimit, PaginationMeta, SortOrder}`        | Shared across all paginated endpoints in `marketfeed`                                     |
| `stock::coinmarketcap::CoinmarketcapEndpoint`                    | `marketfeed::coinmarketcap::CoinmarketcapClient`                        | `CoinmarketcapMirror`, `CoinmarketcapUpdater`, and USD flattening stay in `stock`         |
| `stock::coinmarketcap::quote::QuoteResponse`                     | `marketfeed::coinmarketcap::CoinmarketcapQuotesResponse`                | `stock` converts `CoinmarketcapQuotesResponse` into its sharded `CoinmarketcapQuoteEntry` |
| `stock::oxr::{OxrEndpoint, OxrData}`                             | `marketfeed::oxr::{OxrClient, OxrRates}`                                | `OxrMirror`, `OxrUpdater`, and `TickerPoint` helpers stay in `stock`                      |
| `stock::import::xetra::{load_xetra_csv, XetraEntry}`             | `marketfeed::xetra::{XetraCsvReader, XetraEntry}`                       | `ImportMirror` and `ImportBuilder` stay in `stock`                                        |
| `stock::investing::csv::{InvestingCsvParser, InvestingCsvEntry}` | `marketfeed::investing::{InvestingCsvReader, InvestingEntry}`           | `InvestingImporter` and sharding stay in `stock`                                          |

---

## 5. Implementation Roadmap

### Milestone 1: Core Pagination & Shared Dependencies

- Add runtime dependencies (`chrono`, `csv`, `reqwest` with `blocking` and `json`, `rust_decimal`, `serde`, `serde_json`, `url`) and dev-dependencies (`mockito`, `tempfile`).
- Implement `src/pagination.rs` (`PageLimit`, `PaginationMeta`, `SortOrder`) with full unit test coverage.

### Milestone 2: Online REST API Clients (`marketstack`, `coinmarketcap`, `oxr`)

- Implement `marketfeed::marketstack` (`MarketstackClient`, `MarketstackEodRequest`, `MarketstackEodEntry`, `MarketstackError`) including multi-page fetching and `mockito` integration tests.
- Implement `marketfeed::coinmarketcap` (`CoinmarketcapClient`, `CoinmarketcapQuotesRequest`, `CoinmarketcapQuotesResponse`, `CoinmarketcapError`) returning untransformed multi-currency quote maps with `mockito` tests.
- Implement `marketfeed::oxr` (`OxrClient`, `OxrLatestRequest`, `OxrHistoricalRequest`, `OxrRates`, `OxrError`) with `mockito` tests for latest, historical, and error responses.

### Milestone 3: Exchange CSV Readers (`xetra`, `investing`)

- Implement `marketfeed::xetra` (`XetraCsvReader`, `XetraEntry`, `XetraError`) handling the Xetra preamble line, semicolon delimiter, `%d/%m/%y` dates, and comma-formatted volumes.
- Implement `marketfeed::investing` (`InvestingCsvReader`, `InvestingEntry`, `InvestingError`) handling `%m/%d/%Y` dates, `K`/`M`/`B` volume suffixes, and `-` missing value sentinels without panics or string-slice lint warnings.

### Milestone 4: CLI Subcommands & Documentation

- Implement `src/config.rs` and `src/commands.rs` wiring `marketstack`, `coinmarketcap`, `oxr`, `xetra`, and `investing` subcommands into the `marketfeed` binary with JSON output formatting.
- Add end-to-end CLI tests and update `README.md` with library and CLI usage examples.
