use crate::coinmarketcap::{CoinmarketcapClient, CoinmarketcapError, CoinmarketcapQuotesRequest};
use crate::investing::{InvestingCsvReader, InvestingError};
use crate::marketstack::{MarketstackClient, MarketstackEodRequest, MarketstackError};
use crate::oxr::{OxrAlternativeMode, OxrClient, OxrError, OxrHistoricalRequest, OxrLatestRequest};
use crate::pagination::{PageLimit, SortOrder};
use crate::secrets::{MarketfeedSecrets, MarketfeedSecretsError};
use crate::xetra::{XetraCsvReader, XetraError};
use chrono::NaiveDate;
use clap::{Args, Subcommand};
use std::io;
use std::num::NonZero;
use std::path::PathBuf;
use url::Url;

/// Error type for `marketfeed` CLI command execution.
#[derive(Debug, thiserror::Error)]
pub enum MarketfeedError {
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

/// Top-level command-line arguments for `marketfeed`.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketfeedCommand {
    /// Optional provider subcommand to execute.
    #[command(subcommand)]
    subcommand: Option<ProviderSubcommand>,
}

impl MarketfeedCommand {
    /// Executes the command using secrets from the default configuration path and writes JSON to `stdout`.
    ///
    /// # Errors
    ///
    /// Returns a [`MarketfeedError`] if command execution fails.
    pub fn run(self) -> Result<(), MarketfeedError> {
        let secrets = MarketfeedSecrets::from_default_path()?;
        self.run_with_secrets(&secrets)
    }

    fn run_with_secrets(self, secrets: &MarketfeedSecrets) -> Result<(), MarketfeedError> {
        if let Some(subcommand) = self.subcommand {
            let json_output = subcommand.run(secrets)?;
            println!("{json_output}");
        }
        Ok(())
    }
}

/// Supported market data provider subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ProviderSubcommand {
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
pub enum MarketstackAction {
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
pub struct MarketstackEodCommand {
    /// Comma-separated ticker symbols to query (e.g. `AAPL,MSFT`).
    #[arg(long, value_delimiter = ',', required = true)]
    symbols: Vec<String>,

    /// Optional custom Marketstack base URL.
    #[arg(long)]
    base_url: Option<Url>,

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
    sort: Option<SortOrder>,

    /// Fetch a single page with at most `N` items.
    #[arg(long, conflicts_with_all = ["limit_pages", "all_pages"])]
    limit_items: Option<NonZero<u16>>,

    /// Fetch up to `N` pages and concatenate all returned items.
    #[arg(long, conflicts_with_all = ["limit_items", "all_pages"])]
    limit_pages: Option<NonZero<u8>>,

    /// Fetch all available pages up to the safety ceiling.
    #[arg(long, conflicts_with_all = ["limit_items", "limit_pages"])]
    all_pages: bool,
}

impl MarketstackEodCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let key = secrets
            .marketstack_access_key
            .as_deref()
            .ok_or(MarketstackError::MissingAccessKey)?;
        let client = match self.base_url {
            Some(base_url) => MarketstackClient::with_base_url(key, base_url),
            None => MarketstackClient::new(key)?,
        };

        let mut request = MarketstackEodRequest::new(self.symbols);
        if self.all_pages {
            request = request.with_limit(PageLimit::All);
        } else if let Some(pages) = self.limit_pages {
            request = request.with_limit(PageLimit::Pages(pages));
        } else if let Some(items) = self.limit_items {
            request = request.with_limit(PageLimit::Items(items));
        }
        if let Some(sort) = self.sort {
            request = request.with_sort(sort);
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

        let entries = client.eod(&request)?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }
}

/// Actions supported by the `coinmarketcap` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum CoinmarketcapAction {
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
pub struct CoinmarketcapQuotesCommand {
    /// Comma-separated cryptocurrency symbols to query (e.g. `BTC,ETH`).
    #[arg(long, value_delimiter = ',', required = true)]
    symbols: Vec<String>,

    /// Optional comma-separated target currencies (e.g. `USD,EUR`).
    #[arg(long, value_delimiter = ',')]
    convert: Vec<String>,

    /// Optional custom CoinMarketCap base URL.
    #[arg(long)]
    base_url: Option<Url>,
}

impl CoinmarketcapQuotesCommand {
    fn run(self, secrets: &MarketfeedSecrets) -> Result<String, MarketfeedError> {
        let key = secrets
            .coinmarketcap_api_key
            .as_deref()
            .ok_or(CoinmarketcapError::MissingApiKey)?;
        let client = match self.base_url {
            Some(base_url) => CoinmarketcapClient::with_base_url(key, base_url),
            None => CoinmarketcapClient::new(key)?,
        };

        let mut request = CoinmarketcapQuotesRequest::new(self.symbols);
        if !self.convert.is_empty() {
            request = request.with_convert(self.convert);
        }

        let response = client.quotes_latest(&request)?;
        Ok(serde_json::to_string_pretty(&response)?)
    }
}

/// Actions supported by the `oxr` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum OxrAction {
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
pub struct OxrLatestCommand {
    /// Optional custom OpenExchangeRates base URL.
    #[arg(long)]
    base_url: Option<Url>,

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
            .ok_or(OxrError::MissingAppId)?;
        let client = match self.base_url {
            Some(base_url) => OxrClient::with_base_url(app_id, base_url),
            None => OxrClient::new(app_id)?,
        };

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

        let rates = client.latest(&request)?;
        Ok(serde_json::to_string_pretty(&rates)?)
    }
}

/// CLI arguments for `marketfeed oxr historical`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct OxrHistoricalCommand {
    /// Historical date to query (`YYYY-MM-DD`).
    #[arg(long)]
    date: NaiveDate,

    /// Optional custom OpenExchangeRates base URL.
    #[arg(long)]
    base_url: Option<Url>,

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
            .ok_or(OxrError::MissingAppId)?;
        let client = match self.base_url {
            Some(base_url) => OxrClient::with_base_url(app_id, base_url),
            None => OxrClient::new(app_id)?,
        };

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

        let rates = client.historical(&request)?;
        Ok(serde_json::to_string_pretty(&rates)?)
    }
}

/// Actions supported by the `xetra` subcommand.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum XetraAction {
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
pub struct XetraParseCommand {
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
pub enum InvestingAction {
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
pub struct InvestingParseCommand {
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
    use clap::Parser;
    use googletest::prelude::*;
    use mockito::{Matcher, Server};
    use std::fs;
    use tempfile::TempDir;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(flatten)]
        cmd: MarketfeedCommand,
    }

    #[googletest::test]
    fn run_with_secrets_xetra_and_investing_parse_subcommands_succeed()
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

        let secrets = MarketfeedSecrets::default();
        let xetra_cli = TestCli::try_parse_from([
            "marketfeed",
            "xetra",
            "parse",
            "--file",
            &xetra_path.to_string_lossy(),
        ])?;
        assert_that!(xetra_cli.cmd.run_with_secrets(&secrets), ok(anything()));

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
        assert_that!(investing_cli.cmd.run_with_secrets(&secrets), ok(anything()));
        Ok(())
    }

    #[googletest::test]
    fn run_with_secrets_online_provider_subcommands_via_mock_servers()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let ms_mock = server
            .mock("GET", "/v2/eod")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("access_key".into(), "ms-key".into()),
                Matcher::UrlEncoded("symbols".into(), "AAPL,MSFT".into()),
                Matcher::UrlEncoded("date_from".into(), "2025-01-01".into()),
                Matcher::UrlEncoded("limit".into(), "1000".into()),
                Matcher::UrlEncoded("offset".into(), "0".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"pagination":{"limit":1000,"offset":0,"count":1,"total":1},"data":[{"open":180.0,"high":185.0,"low":179.0,"close":184.0,"volume":1000.0,"symbol":"AAPL","exchange":"XNAS","date":"2025-04-04T00:00:00+0000"}]}"#)
            .create();

        let secrets = MarketfeedSecrets {
            marketstack_access_key: Some("ms-key".to_string()),
            coinmarketcap_api_key: Some("cmc-key".to_string()),
            oxr_app_id: Some("oxr-key".to_string()),
        };

        let ms_cli = TestCli::try_parse_from([
            "marketfeed",
            "marketstack",
            "eod",
            "--symbols",
            "AAPL,MSFT",
            "--date-from",
            "2025-01-01",
            "--all-pages",
            "--base-url",
            &server.url(),
        ])?;
        assert_that!(ms_cli.cmd.run_with_secrets(&secrets), ok(anything()));
        ms_mock.assert();

        let cmc_mock = server
            .mock("GET", "/v2/cryptocurrency/quotes/latest")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("symbol".into(), "BTC,ETH".into()),
                Matcher::UrlEncoded("convert".into(), "USD,EUR".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"status":{"timestamp":"2025-04-04T12:00:00.000Z","error_code":0,"elapsed":5,"credit_count":1},"data":{}}"#)
            .create();

        let cmc_cli = TestCli::try_parse_from([
            "marketfeed",
            "coinmarketcap",
            "quotes",
            "--symbols",
            "BTC,ETH",
            "--convert",
            "USD,EUR",
            "--base-url",
            &server.url(),
        ])?;
        assert_that!(cmc_cli.cmd.run_with_secrets(&secrets), ok(anything()));
        cmc_mock.assert();

        let oxr_mock = server
            .mock("GET", "/api/historical/2025-04-04.json")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("app_id".into(), "oxr-key".into()),
                Matcher::UrlEncoded("show_alternative".into(), "true".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"disclaimer":"d","license":"l","timestamp":1743724799,"base":"USD","rates":{"EUR":0.91}}"#)
            .create();

        let oxr_cli = TestCli::try_parse_from([
            "marketfeed",
            "oxr",
            "historical",
            "--date",
            "2025-04-04",
            "--base-url",
            &server.url(),
        ])?;
        assert_that!(oxr_cli.cmd.run_with_secrets(&secrets), ok(anything()));
        oxr_mock.assert();
        Ok(())
    }

    #[googletest::test]
    fn run_with_secrets_missing_key_returns_provider_error()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let empty_secrets = MarketfeedSecrets::default();

        let ms_cli =
            TestCli::try_parse_from(["marketfeed", "marketstack", "eod", "--symbols", "AAPL"])?;
        assert_that!(
            ms_cli.cmd.run_with_secrets(&empty_secrets),
            err(pat!(MarketfeedError::Marketstack(pat!(
                MarketstackError::MissingAccessKey
            ))))
        );

        let cmc_cli =
            TestCli::try_parse_from(["marketfeed", "coinmarketcap", "quotes", "--symbols", "BTC"])?;
        assert_that!(
            cmc_cli.cmd.run_with_secrets(&empty_secrets),
            err(pat!(MarketfeedError::Coinmarketcap(pat!(
                CoinmarketcapError::MissingApiKey
            ))))
        );

        let oxr_cli = TestCli::try_parse_from(["marketfeed", "oxr", "latest"])?;
        assert_that!(
            oxr_cli.cmd.run_with_secrets(&empty_secrets),
            err(pat!(MarketfeedError::Oxr(pat!(OxrError::MissingAppId))))
        );
        Ok(())
    }
}
