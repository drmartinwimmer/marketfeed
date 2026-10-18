use std::io;

/// Errors that can occur when reading or parsing an Investing.com historical CSV export.
#[derive(Debug, thiserror::Error)]
pub enum InvestingError {
    /// Underlying I/O error while reading the CSV source.
    #[error("I/O error reading Investing.com CSV: {0}")]
    Io(#[from] io::Error),

    /// CSV structural or record deserialization error.
    #[error("CSV error reading Investing.com export: {0}")]
    Csv(#[from] csv::Error),

    /// A data row did not contain the expected 7 comma-delimited columns.
    #[error("invalid Investing.com CSV row at line {line}: expected 7 columns, found {found}")]
    UnexpectedColumnCount {
        /// 1-based data row index.
        line: usize,
        /// Actual number of columns found in the record.
        found: usize,
    },

    /// A date cell could not be parsed as `%m/%d/%Y` or `%Y-%m-%d`.
    #[error("invalid Investing.com date '{value}' at line {line}: {source}")]
    InvalidDate {
        /// 1-based data row index.
        line: usize,
        /// Raw date text from the CSV cell.
        value: String,
        /// Underlying chrono parse error.
        #[source]
        source: chrono::ParseError,
    },

    /// A decimal price, volume, or percentage change cell could not be parsed.
    #[error(
        "invalid Investing.com numeric value '{value}' in column '{field}' at line {line}: {source}"
    )]
    InvalidDecimal {
        /// 1-based data row index.
        line: usize,
        /// Column name (`"Price"`, `"Open"`, `"High"`, `"Low"`, `"Vol."`, or `"Change %"`).
        field: &'static str,
        /// Raw cell text.
        value: String,
        /// Underlying decimal parse error.
        #[source]
        source: rust_decimal::Error,
    },
}
