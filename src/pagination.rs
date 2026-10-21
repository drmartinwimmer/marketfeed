use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZero;

/// Sort order for paginated or time-series market data queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, clap::ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortOrder {
    /// Sort in ascending chronological order (`ASC`).
    #[value(name = "asc", alias = "ASC")]
    Asc,
    /// Sort in descending chronological order (`DESC`).
    #[value(name = "desc", alias = "DESC")]
    Desc,
}

impl SortOrder {
    /// Returns the uppercase query parameter representation (`"ASC"` or `"DESC"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Declarative pagination control for multi-page endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLimit {
    /// Fetch a single page with at most `N` items.
    Items(NonZero<u16>),
    /// Fetch up to `N` pages and concatenate all returned items.
    Pages(NonZero<u8>),
    /// Fetch all available pages up to the endpoint's safety ceiling.
    All,
}

impl PageLimit {
    /// Constructs a [`PageLimit::Items`] limit if `count` is non-zero.
    #[must_use]
    pub const fn items(count: u16) -> Option<Self> {
        match NonZero::new(count) {
            Some(nz) => Some(Self::Items(nz)),
            None => None,
        }
    }

    /// Constructs a [`PageLimit::Pages`] limit if `count` is non-zero.
    #[must_use]
    pub const fn pages(count: u8) -> Option<Self> {
        match NonZero::new(count) {
            Some(nz) => Some(Self::Pages(nz)),
            None => None,
        }
    }
}

/// Pagination metadata returned by offset/limit-paginated endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct PaginationMeta {
    /// Maximum number of items requested per page.
    pub limit: u32,
    /// Zero-based item offset of the current page.
    pub offset: u32,
    /// Number of items returned in the current page.
    pub count: u32,
    /// Total number of items available across all pages.
    pub total: u32,
}

impl PaginationMeta {
    /// Creates a new [`PaginationMeta`] instance.
    #[must_use]
    pub const fn new(limit: u32, offset: u32, count: u32, total: u32) -> Self {
        Self {
            limit,
            offset,
            count,
            total,
        }
    }

    /// Computes the next page's item offset if additional items remain.
    ///
    /// Returns `None` if the current page was empty (`count == 0`) or if
    /// `offset + count >= total`.
    #[must_use]
    pub const fn next_offset(&self) -> Option<u32> {
        if self.count == 0 {
            return None;
        }
        let next = self.offset.saturating_add(self.count);
        if next < self.total { Some(next) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn sort_order_as_str_and_display_return_uppercase_tokens() {
        let asc_token = SortOrder::Asc.as_str();
        let desc_token = SortOrder::Desc.as_str();
        assert_that!(asc_token, eq("ASC"));
        assert_that!(desc_token, eq("DESC"));
        assert_that!(SortOrder::Asc.to_string(), eq("ASC"));
        assert_that!(SortOrder::Desc.to_string(), eq("DESC"));
    }

    #[googletest::test]
    fn page_limit_items_constructor_handles_zero_and_nonzero()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_that!(PageLimit::items(0), none());
        let nonzero = NonZero::new(100_u16).ok_or("expected nonzero")?;
        assert_that!(PageLimit::items(100), some(eq(PageLimit::Items(nonzero))));
        Ok(())
    }

    #[googletest::test]
    fn page_limit_pages_constructor_handles_zero_and_nonzero()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_that!(PageLimit::pages(0), none());
        let nonzero = NonZero::new(3_u8).ok_or("expected nonzero")?;
        assert_that!(PageLimit::pages(3), some(eq(PageLimit::Pages(nonzero))));
        Ok(())
    }

    #[googletest::test]
    fn pagination_meta_next_offset_advances_when_more_items_exist() {
        let meta = PaginationMeta::new(100, 0, 100, 250);
        assert_that!(meta.next_offset(), some(eq(100)));

        let second_page = PaginationMeta::new(100, 100, 100, 250);
        assert_that!(second_page.next_offset(), some(eq(200)));

        let final_page = PaginationMeta::new(100, 200, 50, 250);
        assert_that!(final_page.next_offset(), none());
    }

    #[googletest::test]
    fn pagination_meta_next_offset_returns_none_when_count_is_zero() {
        let empty_page = PaginationMeta::new(100, 0, 0, 250);
        assert_that!(empty_page.next_offset(), none());
    }

    #[googletest::test]
    fn pagination_meta_deserializes_from_json() -> std::result::Result<(), serde_json::Error> {
        let payload = r#"{"limit":100,"offset":200,"count":50,"total":250}"#;
        let meta: PaginationMeta = serde_json::from_str(payload)?;
        assert_that!(meta, eq(PaginationMeta::new(100, 200, 50, 250)));
        Ok(())
    }
}
