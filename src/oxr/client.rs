use crate::oxr::error::{OxrError, OxrServerError};
use crate::oxr::rates::{OxrAlternativeMode, OxrHistoricalRequest, OxrLatestRequest, OxrRates};
use reqwest::blocking::Client;
use url::Url;

/// Synchronous HTTP client for the OpenExchangeRates API.
#[derive(Debug, Clone)]
pub struct OxrClient {
    http: Client,
    app_id: String,
    base_url: Url,
}

impl OxrClient {
    /// Default OpenExchangeRates API base URL (`https://openexchangerates.org`).
    pub const DEFAULT_BASE_URL: &'static str = "https://openexchangerates.org";

    /// Creates a new [`OxrClient`] targeting [`Self::DEFAULT_BASE_URL`].
    ///
    /// # Errors
    ///
    /// Returns [`OxrError::InvalidBaseUrl`] if the default base URL cannot be parsed.
    pub fn new(app_id: impl Into<String>) -> Result<Self, OxrError> {
        let base_url = Url::parse(Self::DEFAULT_BASE_URL)?;
        Ok(Self::with_base_url(app_id, base_url))
    }

    /// Creates a new [`OxrClient`] with a custom `base_url` (e.g. for mock testing).
    #[must_use]
    pub fn with_base_url(app_id: impl Into<String>, base_url: Url) -> Self {
        Self {
            http: Client::new(),
            app_id: app_id.into(),
            base_url,
        }
    }

    /// Queries `/api/latest.json` for the latest currency exchange rates.
    ///
    /// # Errors
    ///
    /// Returns an [`OxrError`] if the HTTP request fails, the payload cannot be deserialized,
    /// or OpenExchangeRates returns an API error.
    pub fn latest(&self, request: &OxrLatestRequest) -> Result<OxrRates, OxrError> {
        self.fetch_rates(
            "api/latest.json",
            request.base.as_deref(),
            &request.symbols,
            request.alternative,
        )
    }

    /// Queries `/api/historical/{YYYY-MM-DD}.json` for historical currency exchange rates.
    ///
    /// # Errors
    ///
    /// Returns an [`OxrError`] if the HTTP request fails, the payload cannot be deserialized,
    /// or OpenExchangeRates returns an API error.
    pub fn historical(&self, request: &OxrHistoricalRequest) -> Result<OxrRates, OxrError> {
        let path = format!("api/historical/{}.json", request.date.format("%Y-%m-%d"));
        self.fetch_rates(
            &path,
            request.base.as_deref(),
            &request.symbols,
            request.alternative,
        )
    }

    fn fetch_rates(
        &self,
        path: &str,
        base: Option<&str>,
        symbols: &[String],
        alternative: OxrAlternativeMode,
    ) -> Result<OxrRates, OxrError> {
        let mut url = self.base_url.join(path)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("app_id", &self.app_id);
            query.append_pair("show_alternative", alternative.as_query_str());
            if let Some(base_code) = base {
                query.append_pair("base", base_code);
            }
            if !symbols.is_empty() {
                query.append_pair("symbols", &symbols.join(","));
            }
        }

        let response = self.http.get(url).send()?;
        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            if let Ok(server_err) = serde_json::from_str::<OxrServerError>(&body) {
                return Err(OxrError::Server(server_err));
            }
            return Err(OxrError::UnexpectedStatus {
                status: status.as_u16(),
                body,
            });
        }

        match serde_json::from_str::<OxrRates>(&body) {
            Ok(rates) => Ok(rates),
            Err(json_err) => {
                if let Ok(server_err) = serde_json::from_str::<OxrServerError>(&body) {
                    return Err(OxrError::Server(server_err));
                }
                Err(OxrError::Json(json_err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use googletest::prelude::*;
    use mockito::{Matcher, Server};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[googletest::test]
    fn latest_fetches_and_deserializes_exchange_rates()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let payload = r#"{
            "disclaimer": "https://openexchangerates.org/terms",
            "license": "https://openexchangerates.org/license",
            "timestamp": 1743768000,
            "base": "USD",
            "rates": {
                "EUR": 0.9125,
                "GBP": 0.785,
                "JPY": 151.42
            }
        }"#;

        let mock = server
            .mock("GET", "/api/latest.json")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("app_id".into(), "oxr-test-id".into()),
                Matcher::UrlEncoded("show_alternative".into(), "true".into()),
                Matcher::UrlEncoded("base".into(), "USD".into()),
                Matcher::UrlEncoded("symbols".into(), "EUR,GBP,JPY".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(payload)
            .create();

        let client = OxrClient::with_base_url("oxr-test-id", Url::parse(&server.url())?);
        let request = OxrLatestRequest::new()
            .with_base("USD")
            .with_symbols(["EUR", "GBP", "JPY"]);

        let rates = client.latest(&request)?;
        mock.assert();

        assert_that!(rates.timestamp, eq(1_743_768_000));
        assert_that!(&rates.base, eq("USD"));
        assert_that!(
            rates.rates.get("EUR"),
            some(eq(&Decimal::from_str("0.9125")?))
        );
        assert_that!(
            rates.rates.get("JPY"),
            some(eq(&Decimal::from_str("151.42")?))
        );
        Ok(())
    }

    #[googletest::test]
    fn historical_fetches_date_specific_endpoint_with_standard_only_mode()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let payload = r#"{
            "disclaimer": "https://openexchangerates.org/terms",
            "license": "https://openexchangerates.org/license",
            "timestamp": 1743724799,
            "base": "USD",
            "rates": {
                "CHF": 0.8995
            }
        }"#;

        let mock = server
            .mock("GET", "/api/historical/2025-04-04.json")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("app_id".into(), "oxr-test-id".into()),
                Matcher::UrlEncoded("show_alternative".into(), "false".into()),
            ]))
            .with_status(200)
            .with_body(payload)
            .create();

        let client = OxrClient::with_base_url("oxr-test-id", Url::parse(&server.url())?);
        let date = NaiveDate::from_ymd_opt(2025, 4, 4).ok_or("invalid date")?;
        let request =
            OxrHistoricalRequest::new(date).with_alternative(OxrAlternativeMode::StandardOnly);

        let rates = client.historical(&request)?;
        mock.assert();

        assert_that!(
            rates.rates.get("CHF"),
            some(eq(&Decimal::from_str("0.8995")?))
        );
        Ok(())
    }

    #[googletest::test]
    fn latest_returns_structured_server_error_on_invalid_app_id()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let err_payload = r#"{
            "error": true,
            "status": 401,
            "message": "invalid_app_id",
            "description": "Invalid App ID provided."
        }"#;

        let mock = server
            .mock("GET", "/api/latest.json")
            .match_query(Matcher::Any)
            .with_status(401)
            .with_body(err_payload)
            .create();

        let client = OxrClient::with_base_url("bad-id", Url::parse(&server.url())?);
        let result = client.latest(&OxrLatestRequest::new());
        mock.assert();

        let Err(OxrError::Server(server_err)) = result else {
            return Err("expected OxrError::Server".into());
        };
        assert_that!(server_err.status, eq(401));
        assert_that!(&server_err.message, eq("invalid_app_id"));
        assert_that!(&server_err.description, eq("Invalid App ID provided."));
        Ok(())
    }
}
