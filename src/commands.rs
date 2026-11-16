use chrono::NaiveDate;
use clap::{Args, Subcommand, ValueEnum};
use marketfeed::coinmarketcap::{
    CoinmarketcapClient, CoinmarketcapError, CoinmarketcapQuotesRequest,
    CoinmarketcapQuotesResponse,
};
use marketfeed::investing::{InvestingCsvReader, InvestingError};
use marketfeed::marketstack::{
    MarketstackClient, MarketstackEodEntry, MarketstackEodRequest, MarketstackError,
};
use marketfeed::oxr::{
    OxrAlternativeMode, OxrClient, OxrError, OxrHistoricalRequest, OxrLatestRequest, OxrRates,
};
use marketfeed::pagination::{PageLimit, SortOrder};
use marketfeed::secrets::{MarketfeedSecrets, MarketfeedSecretsError};
use marketfeed::xetra::{XetraCsvReader, XetraError};
use std::io;
use std::num::NonZero;
use std::path::{Path, PathBuf};

/// Error type for `marketfeed` CLI command execution.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MarketfeedError {
    /// Failed to load provider secrets configuration.
    #[error(transparent)]
    Secrets(#[from] MarketfeedSecretsError),

    /// Marketstack provider error.
    #[error(transparent)]
    Marketstack(#[from] MarketstackError),

    /// CoinMarketCap provider error.
    #[error(transparent)]
    Coinmarketcap(#[from] CoinmarketcapError),

    /// OpenExchangeRates provider error.
    #[error(transparent)]
    Oxr(#[from] OxrError),

    /// Xetra CSV reader error.
    #[error(transparent)]
    Xetra(#[from] XetraError),

    /// Investing.com CSV reader error.
    #[error(transparent)]
    Investing(#[from] InvestingError),

    /// JSON serialization error when formatting CLI output.
    #[error("failed to serialize output as JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error during command execution.
    #[error("I/O error during marketfeed execution: {0}")]
    Io(#[from] io::Error),
}

pub(crate) trait MarketstackBackend {
    fn eod(
        &self,
        request: &MarketstackEodRequest,
    ) -> Result<Vec<MarketstackEodEntry>, MarketstackError>;
}

impl MarketstackBackend for MarketstackClient {
    fn eod(
        &self,
        request: &MarketstackEodRequest,
    ) -> Result<Vec<MarketstackEodEntry>, MarketstackError> {
        MarketstackClient::eod(self, request)
    }
}

pub(crate) trait CoinmarketcapBackend {
    fn quotes_latest(
        &self,
        request: &CoinmarketcapQuotesRequest,
    ) -> Result<CoinmarketcapQuotesResponse, CoinmarketcapError>;
}

impl CoinmarketcapBackend for CoinmarketcapClient {
    fn quotes_latest(
        &self,
        request: &CoinmarketcapQuotesRequest,
    ) -> Result<CoinmarketcapQuotesResponse, CoinmarketcapError> {
        CoinmarketcapClient::quotes_latest(self, request)
    }
}

pub(crate) trait OxrBackend {
    fn latest(&self, request: &OxrLatestRequest) -> Result<OxrRates, OxrError>;
    fn historical(&self, request: &OxrHistoricalRequest) -> Result<OxrRates, OxrError>;
}

impl OxrBackend for OxrClient {
    fn latest(&self, request: &OxrLatestRequest) -> Result<OxrRates, OxrError> {
        OxrClient::latest(self, request)
    }

    fn historical(&self, request: &OxrHistoricalRequest) -> Result<OxrRates, OxrError> {
        OxrClient::historical(self, request)
    }
}

/// CLI sort order flag values mapped onto [`SortOrder`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CliSortOrder {
    /// Sort in ascending chronological order (`ASC`).
    #[value(name = "asc", alias = "ASC")]
    Asc,
    /// Sort in descending chronological order (`DESC`).
    #[value(name = "desc", alias = "DESC")]
    Desc,
}

impl From<CliSortOrder> for SortOrder {
    fn from(value: CliSortOrder) -> Self {
        match value {
            CliSortOrder::Asc => Self::Asc,
            CliSortOrder::Desc => Self::Desc,
        }
    }
}

/// Top-level command-line arguments for `marketfeed`.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MarketfeedCommand {
    /// Optional path to a JSON file containing provider API credentials.
    #[arg(long, global = true)]
    secrets_file: Option<PathBuf>,

    /// Optional provider subcommand to execute.
    #[command(subcommand)]
    subcommand: Option<ProviderSubcommand>,
}

impl MarketfeedCommand {
    /// Executes the command, loading secrets lazily only for online provider subcommands
    /// (from `--secrets-file` if supplied, or from the default configuration path otherwise),
    /// and writes JSON to `stdout`.
    ///
    /// # Errors
    ///
    /// Returns a [`MarketfeedError`] if command execution fails.
    pub(crate) fn run(self) -> Result<(), MarketfeedError> {
        let Some(subcommand) = self.subcommand else {
            return Ok(());
        };
        let secrets = if subcommand.requires_secrets() {
            Self::load_secrets(self.secrets_file.as_deref())?
        } else {
            MarketfeedSecrets::default()
        };
        let json_output = subcommand.run(&secrets)?;
        println!("{json_output}");
        Ok(())
    }

    fn load_secrets(secrets_file: Option<&Path>) -> Result<MarketfeedSecrets, MarketfeedError> {
        match secrets_file {
            Some(path) => Ok(MarketfeedSecrets::from_path(path)?),
            None => Ok(MarketfeedSecrets::from_default_path()?),
        }
    }
}

/// Supported market data provider subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderSubcommand {
    /// Query Marketstack end-of-day (EOD) market data.
    Marketstack {
        /// Marketstack action to perform.
        #[command(subcommand)]
        action: MarketstackAction,
    },
    /// Query CoinMarketCap cryptocurrency quotes.
    Coinmarketcap {
        /// CoinMarketCap action to perform.
        #[command(subcommand)]
        action: CoinmarketcapAction,
    },
    /// Query OpenExchangeRates latest or historical currency rates.
    Oxr {
        /// OpenExchangeRates action to perform.
        #[command(subcommand)]
        action: OxrAction,
    },
    /// Parse Deutsche Börse / Xetra historical CSV files.
    Xetra {
        /// Xetra CSV action to perform.
        #[command(subcommand)]
        action: XetraAction,
    },
    /// Parse Investing.com historical CSV files.
    Investing {
        /// Investing.com CSV action to perform.
        #[command(subcommand)]
        action: InvestingAction,
    },
}

impl ProviderSubcommand {
    const fn requires_secrets(&self) -> bool {
        matches!(
            self,
            Self::Marketstack { .. } | Self::Coinmarketcap { .. } | Self::Oxr { .. }
        )
    }

    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        match self {
            Self::Marketstack { action } => action.run(secrets),
            Self::Coinmarketcap { action } => action.run(secrets),
            Self::Oxr { action } => action.run(secrets),
            Self::Xetra { action } => action.run(),
            Self::Investing { action } => action.run(),
        }
    }
}

/// Actions supported by the `marketstack` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum MarketstackAction {
    /// Query end-of-day (`/v2/eod`) stock data.
    Eod(MarketstackEodCommand),
}

impl MarketstackAction {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        match self {
            Self::Eod(cmd) => cmd.run(secrets),
        }
    }
}

/// CLI arguments for `marketfeed marketstack eod`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarketstackEodCommand {
    /// Comma-separated ticker symbols to query (e.g. `AAPL,MSFT`).
    #[arg(long, value_delimiter = ',', required = true)]
    symbols: Vec<String>,

    /// Inclusive start date (`YYYY-MM-DD`).
    #[arg(long)]
    date_from: Option<NaiveDate>,

    /// Inclusive end date (`YYYY-MM-DD`).
    #[arg(long)]
    date_to: Option<NaiveDate>,

    /// Exchange MIC or code filter (e.g. `XNAS`).
    #[arg(long)]
    exchange: Option<String>,

    /// Sort order (`asc` or `desc`).
    #[arg(long)]
    sort: Option<CliSortOrder>,

    /// Fetch a single page with at most `N` items.
    #[arg(long, conflicts_with_all = ["limit_pages", "all_pages"])]
    limit_items: Option<NonZero<u16>>,

    /// Fetch up to `N` pages and concatenate all returned items.
    #[arg(long, conflicts_with_all = ["limit_items", "all_pages"])]
    limit_pages: Option<NonZero<u8>>,

    /// Fetch all available pages up to the safety ceiling.
    #[arg(long, conflicts_with_all = ["limit_items", "limit_pages"])]
    all_pages: bool,

    /// Override the maximum number of pages fetched when `--all-pages` is enabled (default: 100).
    #[arg(long, requires = "all_pages")]
    all_pages_ceiling: Option<NonZero<u16>>,
}

impl MarketstackEodCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let key = secrets
            .marketstack_access_key
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or(MarketstackError::MissingAccessKey)?;
        let mut client = MarketstackClient::new(key)?;
        if let Some(ceiling) = self.all_pages_ceiling {
            client = client.with_all_pages_ceiling(ceiling);
        }
        self.run_with_backend(&client)
    }

    fn run_with_backend(
        self,
        backend: &impl MarketstackBackend,
    ) -> Result<String, MarketfeedError> {
        let mut request = MarketstackEodRequest::new(self.symbols);
        if self.all_pages {
            request = request.with_limit(PageLimit::All);
        } else if let Some(pages) = self.limit_pages {
            request = request.with_limit(PageLimit::Pages(pages));
        } else if let Some(items) = self.limit_items {
            request = request.with_limit(PageLimit::Items(items));
        }
        if let Some(sort) = self.sort {
            request = request.with_sort(SortOrder::from(sort));
        }
        if let Some(date_from) = self.date_from {
            request = request.with_date_from(date_from);
        }
        if let Some(date_to) = self.date_to {
            request = request.with_date_to(date_to);
        }
        if let Some(exchange) = self.exchange {
            request = request.with_exchange(exchange);
        }

        let entries = backend.eod(&request)?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }
}

/// Actions supported by the `coinmarketcap` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum CoinmarketcapAction {
    /// Query latest cryptocurrency quotes (`/v2/cryptocurrency/quotes/latest`).
    Quotes(CoinmarketcapQuotesCommand),
}

impl CoinmarketcapAction {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        match self {
            Self::Quotes(cmd) => cmd.run(secrets),
        }
    }
}

/// CLI arguments for `marketfeed coinmarketcap quotes`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoinmarketcapQuotesCommand {
    /// Comma-separated cryptocurrency symbols to query (e.g. `BTC,ETH`).
    #[arg(long, value_delimiter = ',', required = true)]
    symbols: Vec<String>,

    /// Optional comma-separated target currencies (e.g. `USD,EUR`).
    #[arg(long, value_delimiter = ',')]
    convert: Vec<String>,
}

impl CoinmarketcapQuotesCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let key = secrets
            .coinmarketcap_api_key
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or(CoinmarketcapError::MissingApiKey)?;
        let client = CoinmarketcapClient::new(key)?;
        self.run_with_backend(&client)
    }

    fn run_with_backend(
        self,
        backend: &impl CoinmarketcapBackend,
    ) -> Result<String, MarketfeedError> {
        let mut request = CoinmarketcapQuotesRequest::new(self.symbols);
        if !self.convert.is_empty() {
            request = request.with_convert(self.convert);
        }

        let response = backend.quotes_latest(&request)?;
        Ok(serde_json::to_string_pretty(&response)?)
    }
}

/// Actions supported by the `oxr` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum OxrAction {
    /// Query latest currency exchange rates (`/api/latest.json`).
    Latest(OxrLatestCommand),
    /// Query historical currency exchange rates (`/api/historical/{date}.json`).
    Historical(OxrHistoricalCommand),
}

impl OxrAction {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        match self {
            Self::Latest(cmd) => cmd.run(secrets),
            Self::Historical(cmd) => cmd.run(secrets),
        }
    }
}

/// CLI arguments for `marketfeed oxr latest`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct OxrLatestCommand {
    /// Optional base currency code (e.g. `USD`).
    #[arg(long)]
    base: Option<String>,

    /// Optional comma-separated currency symbols to filter (e.g. `EUR,GBP,JPY`).
    #[arg(long, value_delimiter = ',')]
    symbols: Vec<String>,

    /// Exclude alternative currencies (`show_alternative=false`).
    #[arg(long)]
    standard_only: bool,
}

impl OxrLatestCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let app_id = secrets
            .oxr_app_id
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or(OxrError::MissingAppId)?;
        let client = OxrClient::new(app_id)?;
        self.run_with_backend(&client)
    }

    fn run_with_backend(self, backend: &impl OxrBackend) -> Result<String, MarketfeedError> {
        let alternative = if self.standard_only {
            OxrAlternativeMode::StandardOnly
        } else {
            OxrAlternativeMode::IncludeAlternative
        };

        let mut request = OxrLatestRequest::new().with_alternative(alternative);
        if let Some(base) = self.base {
            request = request.with_base(base);
        }
        if !self.symbols.is_empty() {
            request = request.with_symbols(self.symbols);
        }

        let rates = backend.latest(&request)?;
        Ok(serde_json::to_string_pretty(&rates)?)
    }
}

/// CLI arguments for `marketfeed oxr historical`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct OxrHistoricalCommand {
    /// Historical date to query (`YYYY-MM-DD`).
    #[arg(long)]
    date: NaiveDate,

    /// Optional base currency code (e.g. `USD`).
    #[arg(long)]
    base: Option<String>,

    /// Optional comma-separated currency symbols to filter (e.g. `EUR,GBP,JPY`).
    #[arg(long, value_delimiter = ',')]
    symbols: Vec<String>,

    /// Exclude alternative currencies (`show_alternative=false`).
    #[arg(long)]
    standard_only: bool,
}

impl OxrHistoricalCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let app_id = secrets
            .oxr_app_id
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or(OxrError::MissingAppId)?;
        let client = OxrClient::new(app_id)?;
        self.run_with_backend(&client)
    }

    fn run_with_backend(self, backend: &impl OxrBackend) -> Result<String, MarketfeedError> {
        let alternative = if self.standard_only {
            OxrAlternativeMode::StandardOnly
        } else {
            OxrAlternativeMode::IncludeAlternative
        };

        let mut request = OxrHistoricalRequest::new(self.date).with_alternative(alternative);
        if let Some(base) = self.base {
            request = request.with_base(base);
        }
        if !self.symbols.is_empty() {
            request = request.with_symbols(self.symbols);
        }

        let rates = backend.historical(&request)?;
        Ok(serde_json::to_string_pretty(&rates)?)
    }
}

/// Actions supported by the `xetra` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum XetraAction {
    /// Parse a local Xetra CSV file and output JSON.
    Parse(XetraParseCommand),
}

impl XetraAction {
    fn run(self) -> Result<String, MarketfeedError> {
        match self {
            Self::Parse(cmd) => cmd.run(),
        }
    }
}

/// CLI arguments for `marketfeed xetra parse`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct XetraParseCommand {
    /// Path to the Xetra CSV file to parse.
    #[arg(long)]
    file: PathBuf,
}

impl XetraParseCommand {
    fn run(self) -> Result<String, MarketfeedError> {
        let entries = XetraCsvReader::read_path(&self.file)?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }
}

/// Actions supported by the `investing` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum InvestingAction {
    /// Parse a local Investing.com CSV file and output JSON.
    Parse(InvestingParseCommand),
}

impl InvestingAction {
    fn run(self) -> Result<String, MarketfeedError> {
        match self {
            Self::Parse(cmd) => cmd.run(),
        }
    }
}

/// CLI arguments for `marketfeed investing parse`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvestingParseCommand {
    /// Path to the Investing.com CSV file to parse.
    #[arg(long)]
    file: PathBuf,
}

impl InvestingParseCommand {
    fn run(self) -> Result<String, MarketfeedError> {
        let entries = InvestingCsvReader::read_path(&self.file)?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use clap::Parser;
    use googletest::prelude::*;
    use marketfeed::coinmarketcap::CoinmarketcapStatus;
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(flatten)]
        cmd: MarketfeedCommand,
    }

    struct FakeMarketstackBackend {
        expected_request: MarketstackEodRequest,
        response: Vec<MarketstackEodEntry>,
    }

    impl MarketstackBackend for FakeMarketstackBackend {
        fn eod(
            &self,
            request: &MarketstackEodRequest,
        ) -> Result<Vec<MarketstackEodEntry>, MarketstackError> {
            assert_that!(request, eq(&self.expected_request));
            Ok(self.response.clone())
        }
    }

    struct FakeCoinmarketcapBackend {
        expected_request: CoinmarketcapQuotesRequest,
        response: CoinmarketcapQuotesResponse,
    }

    impl CoinmarketcapBackend for FakeCoinmarketcapBackend {
        fn quotes_latest(
            &self,
            request: &CoinmarketcapQuotesRequest,
        ) -> Result<CoinmarketcapQuotesResponse, CoinmarketcapError> {
            assert_that!(request, eq(&self.expected_request));
            Ok(self.response.clone())
        }
    }

    struct FakeOxrBackend {
        expected_latest: Option<OxrLatestRequest>,
        expected_historical: Option<OxrHistoricalRequest>,
        response: OxrRates,
    }

    impl OxrBackend for FakeOxrBackend {
        fn latest(&self, request: &OxrLatestRequest) -> Result<OxrRates, OxrError> {
            assert_that!(self.expected_latest.as_ref(), some(eq(request)));
            Ok(self.response.clone())
        }

        fn historical(&self, request: &OxrHistoricalRequest) -> Result<OxrRates, OxrError> {
            assert_that!(self.expected_historical.as_ref(), some(eq(request)));
            Ok(self.response.clone())
        }
    }

    #[googletest::test]
    fn run_xetra_and_investing_parse_subcommands_succeed()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let xetra_path = temp_dir.path().join("ENR.csv");
        fs::write(
            &xetra_path,
            concat!(
                "Xetra;DE000ENER6Y0;\"\";\"\";\"\";\"\";\"\"\n",
                "Date;Open;Close;High;Low;Volume;Volume units\n",
                "04/04/25;42.10;43.50;44.00;41.80;\"1,234,567\";\"28,500\"\n",
            ),
        )?;

        let xetra_cli = TestCli::try_parse_from([
            "marketfeed",
            "xetra",
            "parse",
            "--file",
            &xetra_path.to_string_lossy(),
        ])?;
        assert_that!(xetra_cli.cmd.run(), ok(anything()));

        let investing_path = temp_dir.path().join("GOOG.csv");
        fs::write(
            &investing_path,
            concat!(
                "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n",
                "\"04/04/2025\",\"152.45\",\"151.00\",\"153.20\",\"150.10\",\"12.5K\",\"+1.25%\"\n",
            ),
        )?;

        let investing_cli = TestCli::try_parse_from([
            "marketfeed",
            "investing",
            "parse",
            "--file",
            &investing_path.to_string_lossy(),
        ])?;
        assert_that!(investing_cli.cmd.run(), ok(anything()));
        Ok(())
    }

    #[googletest::test]
    fn run_marketstack_eod_constructs_request_and_formats_json()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let date_from = NaiveDate::from_ymd_opt(2025, 1, 1).ok_or("invalid date")?;
        let fake = FakeMarketstackBackend {
            expected_request: MarketstackEodRequest::new(["AAPL", "MSFT"])
                .with_date_from(date_from)
                .with_sort(SortOrder::Asc)
                .with_limit(PageLimit::All),
            response: vec![MarketstackEodEntry {
                date: Utc::now(),
                open: None,
                adj_open: None,
                close: Some(Decimal::from(184)),
                adj_close: None,
                high: None,
                adj_high: None,
                low: None,
                adj_low: None,
                volume: None,
                adj_volume: None,
                split_factor: None,
                dividend: None,
                name: None,
                symbol: "AAPL".to_string(),
                exchange: "XNAS".to_string(),
                exchange_code: None,
                asset_type: None,
                price_currency: None,
            }],
        };

        let ms_cli = TestCli::try_parse_from([
            "marketfeed",
            "marketstack",
            "eod",
            "--symbols",
            "AAPL,MSFT",
            "--date-from",
            "2025-01-01",
            "--sort",
            "asc",
            "--all-pages",
            "--all-pages-ceiling",
            "1",
        ])?;
        let Some(ProviderSubcommand::Marketstack {
            action: MarketstackAction::Eod(ms_cmd),
        }) = ms_cli.cmd.subcommand
        else {
            return Err("expected marketstack eod".into());
        };

        let ms_json = ms_cmd.run_with_backend(&fake)?;
        assert_that!(&ms_json, contains_substring("\"AAPL\""));
        assert_that!(&ms_json, contains_substring("\"184\""));
        Ok(())
    }

    #[googletest::test]
    fn run_coinmarketcap_quotes_constructs_request_and_formats_json()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let fake = FakeCoinmarketcapBackend {
            expected_request: CoinmarketcapQuotesRequest::new(["BTC", "ETH"])
                .with_convert(["USD", "EUR"]),
            response: CoinmarketcapQuotesResponse {
                status: CoinmarketcapStatus {
                    timestamp: Utc::now(),
                    error_code: 0,
                    error_message: None,
                    elapsed: 5,
                    credit_count: 1,
                    notice: None,
                },
                data: HashMap::new(),
            },
        };

        let cmc_cli = TestCli::try_parse_from([
            "marketfeed",
            "coinmarketcap",
            "quotes",
            "--symbols",
            "BTC,ETH",
            "--convert",
            "USD,EUR",
        ])?;
        let Some(ProviderSubcommand::Coinmarketcap {
            action: CoinmarketcapAction::Quotes(cmc_cmd),
        }) = cmc_cli.cmd.subcommand
        else {
            return Err("expected cmc quotes".into());
        };

        let cmc_json = cmc_cmd.run_with_backend(&fake)?;
        assert_that!(&cmc_json, contains_substring("\"credit_count\": 1"));
        Ok(())
    }

    #[googletest::test]
    fn run_oxr_latest_constructs_request_and_formats_json()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let fake = FakeOxrBackend {
            expected_latest: Some(
                OxrLatestRequest::new()
                    .with_base("USD")
                    .with_symbols(["EUR"])
                    .with_alternative(OxrAlternativeMode::StandardOnly),
            ),
            expected_historical: None,
            response: OxrRates {
                disclaimer: "d".to_string(),
                license: "l".to_string(),
                timestamp: 1_743_724_799,
                base: "USD".to_string(),
                rates: [("EUR".to_string(), Decimal::new(91, 2))]
                    .into_iter()
                    .collect(),
            },
        };

        let oxr_latest_cli = TestCli::try_parse_from([
            "marketfeed",
            "oxr",
            "latest",
            "--base",
            "USD",
            "--symbols",
            "EUR",
            "--standard-only",
        ])?;
        let Some(ProviderSubcommand::Oxr {
            action: OxrAction::Latest(oxr_latest_cmd),
        }) = oxr_latest_cli.cmd.subcommand
        else {
            return Err("expected oxr latest".into());
        };

        let oxr_latest_json = oxr_latest_cmd.run_with_backend(&fake)?;
        assert_that!(&oxr_latest_json, contains_substring("\"EUR\""));
        Ok(())
    }

    #[googletest::test]
    fn run_oxr_historical_constructs_request_and_formats_json()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let expected_date = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        let fake = FakeOxrBackend {
            expected_latest: None,
            expected_historical: Some(OxrHistoricalRequest::new(expected_date)),
            response: OxrRates {
                disclaimer: "d".to_string(),
                license: "l".to_string(),
                timestamp: 1_743_724_799,
                base: "USD".to_string(),
                rates: [("EUR".to_string(), Decimal::new(91, 2))]
                    .into_iter()
                    .collect(),
            },
        };

        let oxr_cli =
            TestCli::try_parse_from(["marketfeed", "oxr", "historical", "--date", "2025-04-04"])?;
        let Some(ProviderSubcommand::Oxr {
            action: OxrAction::Historical(oxr_hist_cmd),
        }) = oxr_cli.cmd.subcommand
        else {
            return Err("expected oxr historical".into());
        };

        let oxr_json = oxr_hist_cmd.run_with_backend(&fake)?;
        assert_that!(&oxr_json, contains_substring("\"EUR\""));
        Ok(())
    }

    #[googletest::test]
    fn run_missing_or_blank_key_returns_provider_error()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let empty_secrets = MarketfeedSecrets::default();

        let ms_cli =
            TestCli::try_parse_from(["marketfeed", "marketstack", "eod", "--symbols", "AAPL"])?;
        let ms_sub = ms_cli.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            ms_sub.run(&empty_secrets),
            err(pat!(MarketfeedError::Marketstack(pat!(
                MarketstackError::MissingAccessKey
            ))))
        );

        let cmc_cli =
            TestCli::try_parse_from(["marketfeed", "coinmarketcap", "quotes", "--symbols", "BTC"])?;
        let cmc_sub = cmc_cli.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            cmc_sub.run(&empty_secrets),
            err(pat!(MarketfeedError::Coinmarketcap(pat!(
                CoinmarketcapError::MissingApiKey
            ))))
        );

        let oxr_cli = TestCli::try_parse_from(["marketfeed", "oxr", "latest"])?;
        let oxr_sub = oxr_cli.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            oxr_sub.run(&empty_secrets),
            err(pat!(MarketfeedError::Oxr(pat!(OxrError::MissingAppId))))
        );

        let whitespace_secrets = MarketfeedSecrets {
            marketstack_access_key: Some("   ".to_string()),
            coinmarketcap_api_key: Some(String::new()),
            oxr_app_id: Some("\t".to_string()),
        };
        let ms_ws =
            TestCli::try_parse_from(["marketfeed", "marketstack", "eod", "--symbols", "AAPL"])?;
        let ms_ws_sub = ms_ws.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            ms_ws_sub.run(&whitespace_secrets),
            err(pat!(MarketfeedError::Marketstack(pat!(
                MarketstackError::MissingAccessKey
            ))))
        );
        let cmc_ws =
            TestCli::try_parse_from(["marketfeed", "coinmarketcap", "quotes", "--symbols", "BTC"])?;
        let cmc_ws_sub = cmc_ws.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            cmc_ws_sub.run(&whitespace_secrets),
            err(pat!(MarketfeedError::Coinmarketcap(pat!(
                CoinmarketcapError::MissingApiKey
            ))))
        );
        let oxr_ws = TestCli::try_parse_from(["marketfeed", "oxr", "latest"])?;
        let oxr_ws_sub = oxr_ws.cmd.subcommand.ok_or("expected subcommand")?;
        assert_that!(
            oxr_ws_sub.run(&whitespace_secrets),
            err(pat!(MarketfeedError::Oxr(pat!(OxrError::MissingAppId))))
        );
        Ok(())
    }

    #[googletest::test]
    fn run_skips_secrets_loading_for_offline_commands_and_loads_secrets_file_for_online()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let missing_secrets = temp_dir.path().join("does-not-exist.json");
        let xetra_path = temp_dir.path().join("ENR.csv");
        fs::write(
            &xetra_path,
            concat!(
                "Date;Open;Close;High;Low;Volume;Volume units\n",
                "04/04/25;42.10;43.50;44.00;41.80;1000;20\n",
            ),
        )?;

        let offline_cli = TestCli::try_parse_from([
            "marketfeed",
            "--secrets-file",
            &missing_secrets.to_string_lossy(),
            "xetra",
            "parse",
            "--file",
            &xetra_path.to_string_lossy(),
        ])?;
        assert_that!(offline_cli.cmd.run(), ok(anything()));

        let online_missing = TestCli::try_parse_from([
            "marketfeed",
            "--secrets-file",
            &missing_secrets.to_string_lossy(),
            "oxr",
            "latest",
        ])?;
        assert_that!(
            online_missing.cmd.run(),
            err(pat!(MarketfeedError::Secrets(pat!(
                MarketfeedSecretsError::Io { .. }
            ))))
        );

        let valid_secrets = temp_dir.path().join("custom-secrets.json");
        fs::write(&valid_secrets, r#"{"marketstack_access_key": "present"}"#)?;
        let online_with_other_key = TestCli::try_parse_from([
            "marketfeed",
            "--secrets-file",
            &valid_secrets.to_string_lossy(),
            "oxr",
            "latest",
        ])?;
        assert_that!(
            online_with_other_key.cmd.run(),
            err(pat!(MarketfeedError::Oxr(pat!(OxrError::MissingAppId))))
        );
        Ok(())
    }
}
