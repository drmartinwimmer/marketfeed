use crate::investing::entry::InvestingEntry;
use crate::investing::error::InvestingError;
use chrono::NaiveDate;
use csv::{ReaderBuilder, StringRecord};
use rust_decimal::Decimal;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::str::FromStr;

/// Stateless CSV reader for Investing.com historical market data exports.
#[derive(Debug, Default, Clone, Copy)]
pub struct InvestingCsvReader;

impl InvestingCsvReader {
    /// Reads and parses an Investing.com CSV file from the given filesystem `path`.
    ///
    /// # Errors
    ///
    /// Returns an [`InvestingError`] if the file cannot be opened or contains malformed rows.
    pub fn read_path(path: &Path) -> Result<Vec<InvestingEntry>, InvestingError> {
        let file = File::open(path)?;
        Self::read(BufReader::new(file))
    }

    /// Parses Investing.com CSV content directly from an in-memory string slice.
    ///
    /// # Errors
    ///
    /// Returns an [`InvestingError`] if the CSV content contains malformed rows.
    pub fn read_str(content: &str) -> Result<Vec<InvestingEntry>, InvestingError> {
        Self::read(Cursor::new(content.as_bytes()))
    }

    /// Reads and parses Investing.com CSV records from any [`Read`] stream.
    ///
    /// # Errors
    ///
    /// Returns an [`InvestingError`] on I/O failure or malformed record fields.
    pub fn read<R: Read>(reader: R) -> Result<Vec<InvestingEntry>, InvestingError> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(reader);

        let mut entries = Vec::new();
        for (idx, record_result) in csv_reader.records().enumerate() {
            let line = idx + 1;
            let record = record_result?;
            if record.iter().all(|cell| cell.trim().is_empty()) {
                continue;
            }
            entries.push(Self::parse_record(&record, line)?);
        }

        Ok(entries)
    }

    fn parse_record(record: &StringRecord, line: usize) -> Result<InvestingEntry, InvestingError> {
        let found = record.len();
        if found < 7 {
            return Err(InvestingError::UnexpectedColumnCount { line, found });
        }

        let date_cell = Self::cell(record, 0, line, found)?;
        let price_cell = Self::cell(record, 1, line, found)?;
        let open_cell = Self::cell(record, 2, line, found)?;
        let high_cell = Self::cell(record, 3, line, found)?;
        let low_cell = Self::cell(record, 4, line, found)?;
        let vol_cell = Self::cell(record, 5, line, found)?;
        let change_cell = Self::cell(record, 6, line, found)?;

        let date = Self::parse_date(date_cell, line)?;
        let price = Self::parse_optional_decimal(price_cell, "Price", line)?;
        let open = Self::parse_optional_decimal(open_cell, "Open", line)?;
        let high = Self::parse_optional_decimal(high_cell, "High", line)?;
        let low = Self::parse_optional_decimal(low_cell, "Low", line)?;
        let volume = Self::parse_optional_volume(vol_cell, line)?;
        let change_percent = Self::parse_optional_change_percent(change_cell, line)?;

        Ok(InvestingEntry {
            date,
            price,
            open,
            high,
            low,
            volume,
            change_percent,
        })
    }

    fn cell(
        record: &StringRecord,
        index: usize,
        line: usize,
        found: usize,
    ) -> Result<&str, InvestingError> {
        record
            .get(index)
            .ok_or(InvestingError::UnexpectedColumnCount { line, found })
    }

    fn parse_date(raw: &str, line: usize) -> Result<NaiveDate, InvestingError> {
        let trimmed = raw.trim().trim_start_matches('\u{feff}');
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%m/%d/%Y") {
            return Ok(date);
        }
        NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").map_err(|source| {
            InvestingError::InvalidDate {
                line,
                value: trimmed.to_string(),
                source,
            }
        })
    }

    fn parse_optional_decimal(
        raw: &str,
        field: &'static str,
        line: usize,
    ) -> Result<Option<Decimal>, InvestingError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            return Ok(None);
        }
        let cleaned = trimmed.replace(',', "");
        let parsed =
            Decimal::from_str(&cleaned).map_err(|source| InvestingError::InvalidDecimal {
                line,
                field,
                value: trimmed.to_string(),
                source,
            })?;
        Ok(Some(parsed))
    }

    fn parse_optional_volume(raw: &str, line: usize) -> Result<Option<Decimal>, InvestingError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            return Ok(None);
        }

        let (number_part, multiplier) = if let Some(prefix) = trimmed.strip_suffix(['K', 'k']) {
            (prefix, Decimal::from(1_000_u64))
        } else if let Some(prefix) = trimmed.strip_suffix(['M', 'm']) {
            (prefix, Decimal::from(1_000_000_u64))
        } else if let Some(prefix) = trimmed.strip_suffix(['B', 'b']) {
            (prefix, Decimal::from(1_000_000_000_u64))
        } else {
            (trimmed, Decimal::ONE)
        };

        let cleaned = number_part.trim().replace(',', "");
        let base =
            Decimal::from_str(&cleaned).map_err(|source| InvestingError::InvalidDecimal {
                line,
                field: "Vol.",
                value: trimmed.to_string(),
                source,
            })?;

        Ok(Some(base * multiplier))
    }

    fn parse_optional_change_percent(
        raw: &str,
        line: usize,
    ) -> Result<Option<Decimal>, InvestingError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            return Ok(None);
        }
        let without_percent = trimmed.strip_suffix('%').unwrap_or(trimmed).trim();
        let without_plus = without_percent.strip_prefix('+').unwrap_or(without_percent);
        let cleaned = without_plus.replace(',', "");
        let parsed =
            Decimal::from_str(&cleaned).map_err(|source| InvestingError::InvalidDecimal {
                line,
                field: "Change %",
                value: trimmed.to_string(),
                source,
            })?;
        Ok(Some(parsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    #[googletest::test]
    fn read_str_parses_k_m_b_volumes_and_missing_sentinels()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let csv = concat!(
            "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n",
            "\"04/04/2025\",\"1,520.45\",\"1,510.00\",\"1,535.20\",\"1,505.10\",\"24.50M\",\"+1.25%\"\n",
            "\"04/03/2025\",\"1,501.68\",\"1,505.00\",\"1,512.00\",\"1,498.00\",\"18.20K\",\"-0.45%\"\n",
            "\"04/02/2025\",\"1,508.47\",\"-\",\"-\",\"-\",\"1.5B\",\"-\"\n",
            "\"04/01/2025\",\"1,500.00\",\"\",\"\",\"\",\"-\",\"0.00%\"\n",
        );

        let entries = InvestingCsvReader::read_str(csv)?;
        assert_that!(entries.len(), eq(4));

        let first = entries.first().ok_or("missing first")?;
        let d1 = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        assert_that!(first.date, eq(d1));
        assert_that!(first.price, some(eq(Decimal::from_str("1520.45")?)));
        assert_that!(first.open, some(eq(Decimal::from_str("1510.00")?)));
        assert_that!(first.high, some(eq(Decimal::from_str("1535.20")?)));
        assert_that!(first.low, some(eq(Decimal::from_str("1505.10")?)));
        assert_that!(first.volume, some(eq(Decimal::from(24_500_000_u64))));
        assert_that!(first.change_percent, some(eq(Decimal::from_str("1.25")?)));

        let second = entries.get(1).ok_or("missing second")?;
        assert_that!(second.volume, some(eq(Decimal::from(18_200_u64))));
        assert_that!(second.change_percent, some(eq(Decimal::from_str("-0.45")?)));

        let third = entries.get(2).ok_or("missing third")?;
        assert_that!(third.open, none());
        assert_that!(third.high, none());
        assert_that!(third.low, none());
        assert_that!(third.volume, some(eq(Decimal::from(1_500_000_000_u64))));
        assert_that!(third.change_percent, none());

        let fourth = entries.get(3).ok_or("missing fourth")?;
        assert_that!(fourth.volume, none());
        assert_that!(fourth.change_percent, some(eq(Decimal::ZERO)));
        Ok(())
    }

    #[googletest::test]
    fn read_path_parses_investing_csv_file() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let temp_dir = TempDir::new()?;
        let csv_path = temp_dir.path().join("investing.csv");
        let csv = concat!(
            "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n",
            "\"04/04/2025\",\"215.10\",\"214.00\",\"216.50\",\"213.80\",\"420\",\"0.51%\"\n",
        );
        fs::write(&csv_path, csv)?;

        let entries = InvestingCsvReader::read_path(&csv_path)?;
        assert_that!(entries.len(), eq(1));
        let first = entries.first().ok_or("missing entry")?;
        assert_that!(first.price, some(eq(Decimal::from_str("215.10")?)));
        assert_that!(first.volume, some(eq(Decimal::from(420_u64))));
        Ok(())
    }

    #[googletest::test]
    fn read_str_reports_errors_on_invalid_date_or_decimal() {
        let bad_date = "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n\"bad\",\"10\",\"10\",\"10\",\"10\",\"10K\",\"1%\"\n";
        assert_that!(
            InvestingCsvReader::read_str(bad_date),
            err(pat!(InvestingError::InvalidDate {
                line: eq(&1_usize),
                value: eq("bad"),
                ..
            }))
        );

        let bad_vol = "\"Date\",\"Price\",\"Open\",\"High\",\"Low\",\"Vol.\",\"Change %\"\n\"04/04/2025\",\"10\",\"10\",\"10\",\"10\",\"abcM\",\"1%\"\n";
        assert_that!(
            InvestingCsvReader::read_str(bad_vol),
            err(pat!(InvestingError::InvalidDecimal {
                line: eq(&1_usize),
                field: eq(&"Vol."),
                value: eq("abcM"),
                ..
            }))
        );
    }
}
