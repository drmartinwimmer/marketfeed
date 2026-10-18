use crate::marketstack::eod::{MarketstackEodEntry, MarketstackEodRequest, MarketstackEodResponse};
use crate::marketstack::error::{MarketstackError, MarketstackErrorEnvelope};
use crate::pagination::PageLimit;
use reqwest::blocking::Client;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolvedPagination {
    page_size: u16,
    max_pages: u8,
}

/// Synchronous HTTP client for the Marketstack REST API (`/v2/eod`).
#[derive(Debug, Clone)]
pub struct MarketstackClient {
    http: Client,
    access_key: String,
    base_url: Url,
}

impl MarketstackClient {
    /// Default Marketstack API base URL (`https://api.marketstack.com`).
    pub const DEFAULT_BASE_URL: &'static str = "https://api.marketstack.com";

    /// Default page size when no [`PageLimit`] is specified on the request.
    pub const DEFAULT_PAGE_SIZE: u16 = 100;

    /// Maximum items per page supported by Marketstack `/v2/eod`.
    pub const MAX_PAGE_SIZE: u16 = 1000;

    /// Safety ceiling on page count when fetching with [`PageLimit::All`].
    pub const ALL_PAGES_SAFETY_CEILING: u8 = 100;

    /// Creates a new [`MarketstackClient`] targeting [`Self::DEFAULT_BASE_URL`].
    ///
    /// # Errors
    ///
    /// Returns [`MarketstackError::InvalidBaseUrl`] if the default base URL cannot be parsed.
    pub fn new(access_key: impl Into<String>) -> Result<Self, MarketstackError> {
        let base_url = Url::parse(Self::DEFAULT_BASE_URL)?;
        Ok(Self::with_base_url(access_key, base_url))
    }

    /// Creates a new [`MarketstackClient`] with a custom `base_url` (e.g. for mock testing).
    #[must_use]
    pub fn with_base_url(access_key: impl Into<String>, base_url: Url) -> Self {
        Self {
            http: Client::new(),
            access_key: access_key.into(),
            base_url,
        }
    }

    /// Queries the `/v2/eod` endpoint and transparently paginates according to `request.limit`.
    ///
    /// # Errors
    ///
    /// Returns a [`MarketstackError`] if `request.symbols` is empty, the requested page limit
    /// exceeds [`Self::MAX_PAGE_SIZE`], the HTTP request fails, or Marketstack returns an error.
    pub fn eod(
        &self,
        request: &MarketstackEodRequest,
    ) -> Result<Vec<MarketstackEodEntry>, MarketstackError> {
        if request.symbols.is_empty() {
            return Err(MarketstackError::MissingSymbols);
        }

        let pagination = Self::resolve_pagination(request.limit)?;
        let mut entries = Vec::new();
        let mut offset = 0_u32;

        for _ in 0..pagination.max_pages {
            let page = self.fetch_eod_page(request, pagination.page_size, offset)?;
            let next_offset = page.pagination.next_offset();
            entries.extend(page.data);
            let Some(next) = next_offset else {
                break;
            };
            offset = next;
        }

        Ok(entries)
    }

    const fn resolve_pagination(
        limit: Option<PageLimit>,
    ) -> Result<ResolvedPagination, MarketstackError> {
        match limit {
            None => Ok(ResolvedPagination {
                page_size: Self::DEFAULT_PAGE_SIZE,
                max_pages: 1,
            }),
            Some(PageLimit::Items(count)) => {
                let raw = count.get();
                if raw > Self::MAX_PAGE_SIZE {
                    Err(MarketstackError::InvalidPaginationLimit {
                        limit: raw,
                        max: Self::MAX_PAGE_SIZE,
                    })
                } else {
                    Ok(ResolvedPagination {
                        page_size: raw,
                        max_pages: 1,
                    })
                }
            }
            Some(PageLimit::Pages(pages)) => Ok(ResolvedPagination {
                page_size: Self::MAX_PAGE_SIZE,
                max_pages: pages.get(),
            }),
            Some(PageLimit::All) => Ok(ResolvedPagination {
                page_size: Self::MAX_PAGE_SIZE,
                max_pages: Self::ALL_PAGES_SAFETY_CEILING,
            }),
        }
    }

    fn fetch_eod_page(
        &self,
        request: &MarketstackEodRequest,
        limit: u16,
        offset: u32,
    ) -> Result<MarketstackEodResponse, MarketstackError> {
        let mut url = self.base_url.join("v2/eod")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("access_key", &self.access_key);
            query.append_pair("symbols", &request.symbols.join(","));
            query.append_pair("limit", &limit.to_string());
            query.append_pair("offset", &offset.to_string());

            if let Some(sort) = request.sort {
                query.append_pair("sort", sort.as_str());
            }
            if let Some(date_from) = request.date_from {
                query.append_pair("date_from", &date_from.format("%Y-%m-%d").to_string());
            }
            if let Some(date_to) = request.date_to {
                query.append_pair("date_to", &date_to.format("%Y-%m-%d").to_string());
            }
            if let Some(exchange) = &request.exchange {
                query.append_pair("exchange", exchange);
            }
        }

        let response = self.http.get(url).send()?;
        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            if let Ok(envelope) = serde_json::from_str::<MarketstackErrorEnvelope>(&body) {
                return Err(MarketstackError::Server(envelope.error));
            }
            return Err(MarketstackError::UnexpectedStatus {
                status: status.as_u16(),
                body,
            });
        }

        match serde_json::from_str::<MarketstackEodResponse>(&body) {
            Ok(parsed) => Ok(parsed),
            Err(json_err) => {
                if let Ok(envelope) = serde_json::from_str::<MarketstackErrorEnvelope>(&body) {
                    return Err(MarketstackError::Server(envelope.error));
                }
                Err(MarketstackError::Json(json_err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pagination::SortOrder;
    use chrono::NaiveDate;
    use googletest::prelude::*;
    use mockito::{Matcher, Server};
    use rust_decimal::Decimal;
    use std::num::NonZero;
    use std::str::FromStr;

    #[googletest::test]
    fn eod_single_page_with_filters_deserializes_entries()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let payload = r#"{
            "pagination": {
                "limit": 50,
                "offset": 0,
                "count": 1,
                "total": 1
            },
            "data": [
                {
                    "open": 188.5,
                    "high": 191.0,
                    "low": 187.25,
                    "close": 190.15,
                    "volume": 45123000.0,
                    "adj_high": 191.0,
                    "adj_low": 187.25,
                    "adj_close": 190.15,
                    "adj_open": 188.5,
                    "adj_volume": 45123000.0,
                    "split_factor": 1.0,
                    "dividend": 0.0,
                    "name": "Apple Inc",
                    "exchange_code": "NASDAQ",
                    "asset_type": "Stock",
                    "price_currency": "USD",
                    "symbol": "AAPL",
                    "exchange": "XNAS",
                    "date": "2025-04-04T00:00:00+0000"
                }
            ]
        }"#;

        let mock = server
            .mock("GET", "/v2/eod")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("access_key".into(), "test-key".into()),
                Matcher::UrlEncoded("symbols".into(), "AAPL,MSFT".into()),
                Matcher::UrlEncoded("limit".into(), "50".into()),
                Matcher::UrlEncoded("offset".into(), "0".into()),
                Matcher::UrlEncoded("sort".into(), "ASC".into()),
                Matcher::UrlEncoded("date_from".into(), "2025-04-01".into()),
                Matcher::UrlEncoded("date_to".into(), "2025-04-04".into()),
                Matcher::UrlEncoded("exchange".into(), "XNAS".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(payload)
            .create();

        let client = MarketstackClient::with_base_url("test-key", Url::parse(&server.url())?);
        let limit = PageLimit::items(50).ok_or("expected nonzero limit")?;
        let date_from = NaiveDate::from_ymd_opt(2025, 4, 1).ok_or("invalid date_from")?;
        let date_to = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date_to")?;

        let request = MarketstackEodRequest::new(["AAPL", "MSFT"])
            .with_limit(limit)
            .with_sort(SortOrder::Asc)
            .with_date_from(date_from)
            .with_date_to(date_to)
            .with_exchange("XNAS");

        let entries = client.eod(&request)?;
        mock.assert();

        assert_that!(entries.len(), eq(1));
        let first = entries.first().ok_or("expected entry")?;
        assert_that!(&first.symbol, eq("AAPL"));
        assert_that!(&first.exchange, eq("XNAS"));
        assert_that!(first.close, eq(Decimal::from_str("190.15")?));
        assert_that!(first.open, eq(Decimal::from_str("188.5")?));
        assert_that!(first.name.as_deref(), some(eq("Apple Inc")));
        assert_that!(first.price_currency.as_deref(), some(eq("USD")));
        Ok(())
    }

    #[googletest::test]
    fn eod_all_pages_fetches_and_concatenates_multiple_pages()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let page1 = r#"{
            "pagination": { "limit": 1000, "offset": 0, "count": 1, "total": 2 },
            "data": [{
                "open": 100.0, "high": 105.0, "low": 99.0, "close": 104.0, "volume": 1000.0,
                "symbol": "AAPL", "exchange": "XNAS", "date": "2025-04-04T00:00:00+0000"
            }]
        }"#;
        let page2 = r#"{
            "pagination": { "limit": 1000, "offset": 1, "count": 1, "total": 2 },
            "data": [{
                "open": 98.0, "high": 101.0, "low": 97.0, "close": 100.0, "volume": 900.0,
                "symbol": "AAPL", "exchange": "XNAS", "date": "2025-04-03T00:00:00+0000"
            }]
        }"#;

        let mock1 = server
            .mock("GET", "/v2/eod")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("access_key".into(), "key".into()),
                Matcher::UrlEncoded("symbols".into(), "AAPL".into()),
                Matcher::UrlEncoded("limit".into(), "1000".into()),
                Matcher::UrlEncoded("offset".into(), "0".into()),
            ]))
            .with_status(200)
            .with_body(page1)
            .create();

        let mock2 = server
            .mock("GET", "/v2/eod")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("access_key".into(), "key".into()),
                Matcher::UrlEncoded("symbols".into(), "AAPL".into()),
                Matcher::UrlEncoded("limit".into(), "1000".into()),
                Matcher::UrlEncoded("offset".into(), "1".into()),
            ]))
            .with_status(200)
            .with_body(page2)
            .create();

        let client = MarketstackClient::with_base_url("key", Url::parse(&server.url())?);
        let request = MarketstackEodRequest::new(["AAPL"]).with_limit(PageLimit::All);
        let entries = client.eod(&request)?;

        mock1.assert();
        mock2.assert();
        assert_that!(entries.len(), eq(2));
        let first = entries.first().ok_or("missing first")?;
        let second = entries.get(1).ok_or("missing second")?;
        assert_that!(first.close, eq(Decimal::from(104)));
        assert_that!(second.close, eq(Decimal::from(100)));
        Ok(())
    }

    #[googletest::test]
    fn eod_rejects_empty_symbols_and_excessive_item_limit()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let client = MarketstackClient::new("key")?;

        let empty_req = MarketstackEodRequest::default();
        let empty_err = client.eod(&empty_req);
        assert_that!(empty_err, err(pat!(MarketstackError::MissingSymbols)));

        let too_large = NonZero::new(1001_u16).ok_or("nonzero")?;
        let large_req =
            MarketstackEodRequest::new(["AAPL"]).with_limit(PageLimit::Items(too_large));
        let limit_err = client.eod(&large_req);
        assert_that!(
            limit_err,
            err(pat!(MarketstackError::InvalidPaginationLimit {
                limit: eq(&1001_u16),
                max: eq(&1000_u16),
            }))
        );
        Ok(())
    }

    #[googletest::test]
    fn eod_returns_structured_server_error_on_api_failure()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let err_payload = r#"{
            "error": {
                "code": "invalid_access_key",
                "message": "You have not supplied a valid API Access Key.",
                "context": { "access_key": ["invalid"] }
            }
        }"#;

        let mock = server
            .mock("GET", "/v2/eod")
            .match_query(Matcher::Any)
            .with_status(401)
            .with_body(err_payload)
            .create();

        let client = MarketstackClient::with_base_url("bad-key", Url::parse(&server.url())?);
        let request = MarketstackEodRequest::new(["AAPL"]);
        let result = client.eod(&request);
        mock.assert();

        let Err(MarketstackError::Server(server_err)) = result else {
            return Err("expected MarketstackError::Server".into());
        };
        assert_that!(&server_err.code, eq("invalid_access_key"));
        assert_that!(
            &server_err.message,
            contains_substring("valid API Access Key")
        );
        assert_that!(server_err.context, some(anything()));
        Ok(())
    }
}
