use std::io;

/// Errors that can occur when reading or parsing a Deutsche Börse / Xetra CSV export.
#[derive(Debug, thiserror::Error)]
pub enum XetraError {
    /// Underlying I/O error while reading the CSV source.
    #[error("I/O error reading Xetra CSV: {0}")]
    Io(#[from] io::Error),

    /// CSV structural or record deserialization error.
    #[error("CSV error reading Xetra export: {0}")]
    Csv(#[from] csv::Error),

    /// A data row did not contain the expected 7 semicolon-delimited columns.
    #[error("invalid Xetra CSV row at line {line}: expected 7 columns, found {found}")]
    UnexpectedColumnCount {
        /// 1-based data row index.
        line: usize,
        /// Actual number of columns found in the record.
        found: usize,
    },

    /// A date cell could not be parsed as `%d/%m/%y` or `%d/%m/%Y`.
    #[error("invalid Xetra date '{value}' at line {line}: {source}")]
    InvalidDate {
        /// 1-based data row index.
        line: usize,
        /// Raw date text from the CSV cell.
        value: String,
        /// Underlying chrono parse error.
        #[source]
        source: chrono::ParseError,
    },

    /// A decimal price field (`open`, `close`, `high`, or `low`) could not be parsed.
    #[error("invalid Xetra decimal '{value}' in column '{field}' at line {line}: {source}")]
    InvalidDecimal {
        /// 1-based data row index.
        line: usize,
        /// Column name (`"open"`, `"close"`, `"high"`, or `"low"`).
        field: &'static str,
        /// Raw cell text.
        value: String,
        /// Underlying decimal parse error.
        #[source]
        source: rust_decimal::Error,
    },

    /// An integer volume field (`volume` or `volume_units`) could not be parsed.
    #[error("invalid Xetra volume '{value}' in column '{field}' at line {line}: {source}")]
    InvalidVolume {
        /// 1-based data row index.
        line: usize,
        /// Column name (`"volume"` or `"volume_units"`).
        field: &'static str,
        /// Raw cell text.
        value: String,
        /// Underlying integer parse error.
        #[source]
        source: std::num::ParseIntError,
    },
}
