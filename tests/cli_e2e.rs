use chrono::NaiveDate;
use googletest::prelude::*;
use marketfeed::investing::InvestingEntry;
use marketfeed::xetra::XetraEntry;
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

    for args in [
        &["marketstack", "eod", "--help"][..],
        &["coinmarketcap", "quotes", "--help"][..],
        &["oxr", "latest", "--help"][..],
        &["oxr", "historical", "--help"][..],
    ] {
        let sub_help = marketfeed_cmd().args(args).output()?;
        assert_that!(sub_help.status.code(), some(eq(0)));
        let sub_help_text = String::from_utf8(sub_help.stdout)?;
        assert_that!(&sub_help_text, not(contains_substring("--base-url")));
    }

    let rejected_flag = marketfeed_cmd()
        .args([
            "marketstack",
            "eod",
            "--symbols",
            "AAPL",
            "--base-url",
            "https://example.invalid",
        ])
        .output()?;
    assert_that!(rejected_flag.status.success(), is_false());
    let rejected_stderr = String::from_utf8(rejected_flag.stderr)?;
    assert_that!(&rejected_stderr, contains_substring("--base-url"));
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
fn online_subcommands_resolve_custom_and_xdg_secrets_files()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let custom_secrets = temp_dir.path().join("custom-secrets.json");
    fs::write(
        &custom_secrets,
        r#"{"marketstack_access_key": "present-ms-key"}"#,
    )?;

    let cmc_out = marketfeed_cmd()
        .arg("--secrets-file")
        .arg(&custom_secrets)
        .args(["coinmarketcap", "quotes", "--symbols", "BTC"])
        .output()?;
    assert_that!(cmc_out.status.code(), some(eq(2)));
    let cmc_stderr = String::from_utf8(cmc_out.stderr)?;
    assert_that!(
        &cmc_stderr,
        contains_substring("Error: missing CoinMarketCap API key")
    );

    let default_cfg_dir = temp_dir.path().join("xdg").join("marketfeed");
    fs::create_dir_all(&default_cfg_dir)?;
    fs::write(
        default_cfg_dir.join("secrets.json"),
        r#"{"coinmarketcap_api_key": "present-cmc-key"}"#,
    )?;

    let oxr_out = marketfeed_cmd()
        .env("HOME", temp_dir.path().join("xdg"))
        .env("XDG_CONFIG_HOME", temp_dir.path().join("xdg"))
        .args(["oxr", "latest", "--standard-only"])
        .output()?;
    assert_that!(oxr_out.status.code(), some(eq(2)));
    let oxr_stderr = String::from_utf8(oxr_out.stderr)?;
    assert_that!(
        &oxr_stderr,
        contains_substring("Error: missing OpenExchangeRates App ID")
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
