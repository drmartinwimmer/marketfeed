//! Deutsche Börse / Xetra historical CSV reader and entry types.

mod entry;
mod error;
mod reader;

pub use entry::XetraEntry;
pub use error::XetraError;
pub use reader::XetraCsvReader;
