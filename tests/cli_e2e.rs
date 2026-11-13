use chrono::NaiveDate;
use googletest::prelude::*;
use marketfeed::coinmarketcap::CoinmarketcapQuotesResponse;
use marketfeed::investing::InvestingEntry;
use marketfeed::marketstack::MarketstackEodEntry;
use marketfeed::oxr::OxrRates;
use marketfeed::xetra::XetraEntry;
use mockito::{Matcher, Server};
use rust_decimal::Decimal;
use std::fs;
use std::process::Command;
use std::str::FromStr;
use tempfile::TempDir;

fn marketfeed_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_marketfeed"))
}

#[googletest::test]
fn run_cli_without_subcommand_and_with_help_succeeds()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let default_out = marketfeed_cmd()
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .output()?;
    assert_that!(default_out.status.code(), some(eq(0)));
    assert_that!(default_out.stdout.is_empty(), is_true());
    assert_that!(default_out.stderr.is_empty(), is_true());

    let help_out = marketfeed_cmd().arg("--help").output()?;
    assert_that!(help_out.status.code(), some(eq(0)));
    let help_text = String::from_utf8(help_out.stdout)?;
    assert_that!(&help_text, contains_substring("marketstack"));
    assert_that!(&help_text, contains_substring("coinmarketcap"));
    assert_that!(&help_text, contains_substring("oxr"));
    assert_that!(&help_text, contains_substring("xetra"));
    assert_that!(&help_text, contains_substring("investing"));
    assert_that!(&help_text, contains_substring("--secrets-file"));
    Ok(())
}

#[googletest::test]
fn parse_csv_subcommands_output_typed_json_without_loading_broken_default_secrets()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("marketfeed");
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join("secrets.json"), "{ invalid json")?;

    let xetra_file = temp_dir.path().join("ENR.csv");
    fs::write(
        &xetra_file,
        concat!(
            "Xetra;DE000ENER6Y0;\"\";\"\";\"\";\"\";\"\"\n",
            "Date;Open;Close;High;Low;Volume;Volume units\n",
            "04/04/25;42.10;43.50;44.00;41.80;\"1,234,567\";\"28,500\"\n",
        ),
    )?;

    let xetra_out = marketfeed_cmd()
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .args(["xetra", "parse", "--file"])
        .arg(&xetra_file)
        .output()?;
    assert_that!(xetra_out.status.code(), some(eq(0)));
    assert_that!(xetra_out.stderr.is_empty(), is_true());

    let xetra_entries: Vec<XetraEntry> = serde_json::from_slice(&xetra_out.stdout)?;
    assert_that!(xetra_entries.len(), eq(1));
    let first_xetra = xetra_entries.first().ok_or("missing xetra entry")?;
    let expected_date = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
    assert_that!(first_xetra.date, eq(expected_date));
    assert_that!(first_xetra.close, eq(Decimal::from_str("43.50")?));
    assert_that!(first_xetra.volume, eq(1_234_567));
    assert_that!(first_xetra.volume_units, eq(28_500));

    let investing_file = temp_dir.path().join("GOOG.csv");
    fs::write(
        &investing_file,
        concat!(
            "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n",
            "\"04/04/2025\",\"152.45\",\"151.00\",\"153.20\",\"150.10\",\"12.5K\",\"+1.25%\"\n",
        ),
    )?;

    let investing_out = marketfeed_cmd()
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .args(["investing", "parse", "--file"])
        .arg(&investing_file)
        .output()?;
    assert_that!(investing_out.status.code(), some(eq(0)));
    assert_that!(investing_out.stderr.is_empty(), is_true());

    let investing_entries: Vec<InvestingEntry> = serde_json::from_slice(&investing_out.stdout)?;
    assert_that!(investing_entries.len(), eq(1));
    let first_investing = investing_entries.first().ok_or("missing investing entry")?;
    assert_that!(first_investing.date, eq(expected_date));
    assert_that!(
        first_investing.price,
        some(eq(Decimal::from_str("152.45")?))
    );
    assert_that!(first_investing.volume, some(eq(Decimal::from(12_500_u64))));
    assert_that!(
        first_investing.change_percent,
        some(eq(Decimal::from_str("1.25")?))
    );
    Ok(())
}

#[googletest::test]
fn query_online_providers_end_to_end_outputs_valid_json()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let secrets_file = temp_dir.path().join("custom-secrets.json");
    fs::write(
        &secrets_file,
        r#"{
            "marketstack_access_key": "e2e-ms-key",
            "coinmarketcap_api_key": "e2e-cmc-key",
            "oxr_app_id": "e2e-oxr-id"
        }"#,
    )?;

    let mut server = Server::new();
    let ms_mock = server
        .mock("GET", "/v2/eod")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("access_key".into(), "e2e-ms-key".into()),
            Matcher::UrlEncoded("symbols".into(), "AAPL".into()),
            Matcher::UrlEncoded("limit".into(), "1000".into()),
            Matcher::UrlEncoded("offset".into(), "0".into()),
        ]))
        .with_status(200)
        .with_body(r#"{"pagination":{"limit":1000,"offset":0,"count":1,"total":4},"data":[{"open":188.5,"high":191.0,"low":187.25,"close":190.15,"volume":45000.0,"symbol":"AAPL","exchange":"XNAS","date":"2025-04-04T00:00:00+0000"}]}"#)
        .create();

    let ms_out = marketfeed_cmd()
        .arg("--secrets-file")
        .arg(&secrets_file)
        .args([
            "marketstack",
            "eod",
            "--symbols",
            "AAPL",
            "--all-pages",
            "--all-pages-ceiling",
            "1",
            "--base-url",
            &server.url(),
        ])
        .output()?;
    ms_mock.assert();
    assert_that!(ms_out.status.code(), some(eq(0)));
    assert_that!(ms_out.stderr.is_empty(), is_true());

    let ms_entries: Vec<MarketstackEodEntry> = serde_json::from_slice(&ms_out.stdout)?;
    assert_that!(ms_entries.len(), eq(1));
    let first_ms = ms_entries.first().ok_or("missing marketstack entry")?;
    assert_that!(&first_ms.symbol, eq("AAPL"));
    assert_that!(first_ms.close, eq(Decimal::from_str("190.15")?));

    let cmc_mock = server
        .mock("GET", "/v2/cryptocurrency/quotes/latest")
        .match_header("X-CMC_PRO_API_KEY", "e2e-cmc-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTC".into()),
            Matcher::UrlEncoded("convert".into(), "USD".into()),
        ]))
        .with_status(200)
        .with_body(r#"{"status":{"timestamp":"2025-04-04T12:00:00.000Z","error_code":0,"elapsed":5,"credit_count":1},"data":{"BTC":[{"id":1,"name":"Bitcoin","symbol":"BTC","slug":"bitcoin","quote":{"USD":{"price":84250.5}}}]}}"#)
        .create();

    let cmc_out = marketfeed_cmd()
        .arg("--secrets-file")
        .arg(&secrets_file)
        .args([
            "coinmarketcap",
            "quotes",
            "--symbols",
            "BTC",
            "--convert",
            "USD",
            "--base-url",
            &server.url(),
        ])
        .output()?;
    cmc_mock.assert();
    assert_that!(cmc_out.status.code(), some(eq(0)));
    let cmc_resp: CoinmarketcapQuotesResponse = serde_json::from_slice(&cmc_out.stdout)?;
    let btc = cmc_resp
        .data
        .get("BTC")
        .and_then(|items| items.first())
        .ok_or("missing BTC quote")?;
    assert_that!(&btc.name, eq("Bitcoin"));

    // Also verify default `<XDG_CONFIG_HOME>/marketfeed/secrets.json` resolution without `--secrets-file`
    let default_cfg_dir = temp_dir.path().join("xdg").join("marketfeed");
    fs::create_dir_all(&default_cfg_dir)?;
    fs::write(
        default_cfg_dir.join("secrets.json"),
        r#"{"oxr_app_id": "xdg-oxr-id"}"#,
    )?;

    let oxr_mock = server
        .mock("GET", "/api/latest.json")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("app_id".into(), "xdg-oxr-id".into()),
            Matcher::UrlEncoded("show_alternative".into(), "false".into()),
        ]))
        .with_status(200)
        .with_body(r#"{"disclaimer":"d","license":"l","timestamp":1743768000,"base":"USD","rates":{"EUR":0.9125}}"#)
        .create();

    let oxr_out = marketfeed_cmd()
        .env("HOME", temp_dir.path().join("xdg"))
        .env("XDG_CONFIG_HOME", temp_dir.path().join("xdg"))
        .args([
            "oxr",
            "latest",
            "--standard-only",
            "--base-url",
            &server.url(),
        ])
        .output()?;
    oxr_mock.assert();
    assert_that!(oxr_out.status.code(), some(eq(0)));
    let oxr_rates: OxrRates = serde_json::from_slice(&oxr_out.stdout)?;
    assert_that!(
        oxr_rates.rates.get("EUR"),
        some(eq(&Decimal::from_str("0.9125")?))
    );
    Ok(())
}

#[googletest::test]
fn run_cli_on_errors_writes_stderr_and_returns_exit_code_two()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let empty_secrets = temp_dir.path().join("empty-secrets.json");
    fs::write(&empty_secrets, r#"{"marketstack_access_key": "   "}"#)?;

    let missing_key_out = marketfeed_cmd()
        .arg("--secrets-file")
        .arg(&empty_secrets)
        .args(["marketstack", "eod", "--symbols", "AAPL"])
        .output()?;
    assert_that!(missing_key_out.status.code(), some(eq(2)));
    assert_that!(missing_key_out.stdout.is_empty(), is_true());
    let stderr_text = String::from_utf8(missing_key_out.stderr)?;
    assert_that!(
        &stderr_text,
        contains_substring("Error: missing Marketstack access key")
    );

    let missing_csv = temp_dir.path().join("nonexistent.csv");
    let csv_err_out = marketfeed_cmd()
        .args(["xetra", "parse", "--file"])
        .arg(&missing_csv)
        .output()?;
    assert_that!(csv_err_out.status.code(), some(eq(2)));
    assert_that!(csv_err_out.stdout.is_empty(), is_true());
    let csv_stderr = String::from_utf8(csv_err_out.stderr)?;
    assert_that!(&csv_stderr, contains_substring("Error:"));
    Ok(())
}
