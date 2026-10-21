use crate::xetra::entry::XetraEntry;
use crate::xetra::error::XetraError;
use chrono::NaiveDate;
use csv::{ReaderBuilder, StringRecord};
use rust_decimal::Decimal;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::str::FromStr;

/// Stateless CSV reader for Deutsche Börse / Xetra historical market data exports.
#[derive(Debug, Default, Clone, Copy)]
pub struct XetraCsvReader;

impl XetraCsvReader {
    /// Reads and parses a Xetra CSV file from the given filesystem `path`.
    ///
    /// # Errors
    ///
    /// Returns a [`XetraError`] if the file cannot be opened or contains malformed rows.
    pub fn read_path(path: &Path) -> Result<Vec<XetraEntry>, XetraError> {
        let file = File::open(path)?;
        Self::read(BufReader::new(file))
    }

    /// Parses Xetra CSV content directly from an in-memory string slice.
    ///
    /// # Errors
    ///
    /// Returns a [`XetraError`] if the CSV content contains malformed rows.
    pub fn read_str(content: &str) -> Result<Vec<XetraEntry>, XetraError> {
        Self::read(Cursor::new(content.as_bytes()))
    }

    /// Reads and parses Xetra CSV records from any [`BufRead`] stream.
    ///
    /// Automatically skips the optional `Xetra;<ISIN>;"";"";"";"";""` preamble line
    /// as well as the semicolon-delimited column header line.
    ///
    /// # Errors
    ///
    /// Returns a [`XetraError`] on I/O failure or malformed record fields.
    pub fn read<R: BufRead>(mut reader: R) -> Result<Vec<XetraEntry>, XetraError> {
        let mut line_buf = String::new();
        loop {
            line_buf.clear();
            let bytes_read = reader.read_line(&mut line_buf)?;
            if bytes_read == 0 {
                return Ok(Vec::new());
            }
            let trimmed = line_buf.trim().trim_start_matches('\u{feff}');
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("Xetra;") {
                let mut header_buf = String::new();
                let header_bytes = reader.read_line(&mut header_buf)?;
                if header_bytes == 0 {
                    return Ok(Vec::new());
                }
            }
            break;
        }

        let mut csv_reader = ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
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

    fn parse_record(record: &StringRecord, line: usize) -> Result<XetraEntry, XetraError> {
        let found = record.len();
        if found < 7 {
            return Err(XetraError::UnexpectedColumnCount { line, found });
        }

        let date_cell = Self::cell(record, 0, line, found)?;
        let open_cell = Self::cell(record, 1, line, found)?;
        let close_cell = Self::cell(record, 2, line, found)?;
        let high_cell = Self::cell(record, 3, line, found)?;
        let low_cell = Self::cell(record, 4, line, found)?;
        let vol_cell = Self::cell(record, 5, line, found)?;
        let units_cell = Self::cell(record, 6, line, found)?;

        let date = Self::parse_date(date_cell, line)?;
        let open = Self::parse_decimal(open_cell, "open", line)?;
        let close = Self::parse_decimal(close_cell, "close", line)?;
        let high = Self::parse_decimal(high_cell, "high", line)?;
        let low = Self::parse_decimal(low_cell, "low", line)?;
        let volume = Self::parse_volume(vol_cell, "volume", line)?;
        let volume_units = Self::parse_volume(units_cell, "volume_units", line)?;

        Ok(XetraEntry {
            date,
            open,
            close,
            high,
            low,
            volume,
            volume_units,
        })
    }

    fn cell(
        record: &StringRecord,
        index: usize,
        line: usize,
        found: usize,
    ) -> Result<&str, XetraError> {
        record
            .get(index)
            .ok_or(XetraError::UnexpectedColumnCount { line, found })
    }

    fn parse_date(raw: &str, line: usize) -> Result<NaiveDate, XetraError> {
        let trimmed = raw.trim();
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d/%m/%y") {
            return Ok(date);
        }
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
            return Ok(date);
        }
        NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").map_err(|source| XetraError::InvalidDate {
            line,
            value: trimmed.to_string(),
            source,
        })
    }

    fn parse_decimal(raw: &str, field: &'static str, line: usize) -> Result<Decimal, XetraError> {
        let cleaned = raw.trim().replace(',', "");
        Decimal::from_str(&cleaned).map_err(|source| XetraError::InvalidDecimal {
            line,
            field,
            value: raw.trim().to_string(),
            source,
        })
    }

    fn parse_volume(raw: &str, field: &'static str, line: usize) -> Result<u64, XetraError> {
        let cleaned = raw.trim().replace(',', "");
        u64::from_str(&cleaned).map_err(|source| XetraError::InvalidVolume {
            line,
            field,
            value: raw.trim().to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    #[googletest::test]
    fn read_str_parses_xetra_csv_with_preamble_and_comma_volumes()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let csv = concat!(
            "Xetra;DE000ENER6Y0;\"\";\"\";\"\";\"\";\"\"\n",
            "Date;Open;Close;High;Low;Volume;Volume units\n",
            "04/04/25;42.10;43.50;44.00;41.80;\"1,234,567\";\"28,500\"\n",
            "03/04/25;41.00;42.05;42.30;40.90;890123;21000\n",
        );

        let entries = XetraCsvReader::read_str(csv)?;
        assert_that!(entries.len(), eq(2));

        let first = entries.first().ok_or("missing first entry")?;
        let expected_date = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        assert_that!(first.date, eq(expected_date));
        assert_that!(first.open, eq(Decimal::from_str("42.10")?));
        assert_that!(first.close, eq(Decimal::from_str("43.50")?));
        assert_that!(first.high, eq(Decimal::from_str("44.00")?));
        assert_that!(first.low, eq(Decimal::from_str("41.80")?));
        assert_that!(first.volume, eq(1_234_567));
        assert_that!(first.volume_units, eq(28_500));
        Ok(())
    }

    #[googletest::test]
    fn read_path_parses_file_without_preamble_line()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let csv_path = temp_dir.path().join("xetra.csv");
        let csv = concat!(
            "Date;Open;Close;High;Low;Volume;Volume units\n",
            "04/04/2025;100.00;102.50;103.00;99.50;\"500,000\";\"4,900\"\n",
        );
        fs::write(&csv_path, csv)?;

        let entries = XetraCsvReader::read_path(&csv_path)?;
        assert_that!(entries.len(), eq(1));
        let first = entries.first().ok_or("missing entry")?;
        assert_that!(first.close, eq(Decimal::from_str("102.50")?));
        assert_that!(first.volume, eq(500_000));
        assert_that!(first.volume_units, eq(4_900));
        Ok(())
    }

    #[googletest::test]
    fn read_str_reports_errors_on_malformed_rows() {
        let short_row = "Date;Open;Close;High;Low;Volume;Volume units\n04/04/25;42.10;43.50\n";
        assert_that!(
            XetraCsvReader::read_str(short_row),
            err(pat!(XetraError::UnexpectedColumnCount {
                line: eq(&1_usize),
                found: eq(&3_usize),
            }))
        );

        let bad_date = "Date;Open;Close;High;Low;Volume;Volume units\nnot-a-date;42.10;43.50;44.00;41.80;1000;20\n";
        assert_that!(
            XetraCsvReader::read_str(bad_date),
            err(pat!(XetraError::InvalidDate {
                line: eq(&1_usize),
                value: eq("not-a-date"),
                ..
            }))
        );
    }
}
