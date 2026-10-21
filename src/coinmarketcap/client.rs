use crate::coinmarketcap::error::{CoinmarketcapError, CoinmarketcapErrorEnvelope};
use crate::coinmarketcap::quote::{CoinmarketcapQuotesRequest, CoinmarketcapQuotesResponse};
use reqwest::blocking::Client;
use reqwest::header::ACCEPT;
use url::Url;

/// Synchronous HTTP client for the CoinMarketCap Pro API (`/v2/cryptocurrency/quotes/latest`).
#[derive(Debug, Clone)]
pub struct CoinmarketcapClient {
    http: Client,
    api_key: String,
    base_url: Url,
}

impl CoinmarketcapClient {
    /// Default CoinMarketCap Pro API base URL (`https://pro-api.coinmarketcap.com`).
    pub const DEFAULT_BASE_URL: &'static str = "https://pro-api.coinmarketcap.com";

    /// Header name used to authenticate CoinMarketCap Pro API requests.
    pub const API_KEY_HEADER: &'static str = "X-CMC_PRO_API_KEY";

    /// Creates a new [`CoinmarketcapClient`] targeting [`Self::DEFAULT_BASE_URL`].
    ///
    /// # Errors
    ///
    /// Returns [`CoinmarketcapError::InvalidBaseUrl`] if the default base URL cannot be parsed.
    pub fn new(api_key: impl Into<String>) -> Result<Self, CoinmarketcapError> {
        let base_url = Url::parse(Self::DEFAULT_BASE_URL)?;
        Ok(Self::with_base_url(api_key, base_url))
    }

    /// Creates a new [`CoinmarketcapClient`] with a custom `base_url` (e.g. for mock testing).
    #[must_use]
    pub fn with_base_url(api_key: impl Into<String>, base_url: Url) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
            base_url,
        }
    }

    /// Queries `/v2/cryptocurrency/quotes/latest` and returns the untransformed response.
    ///
    /// # Errors
    ///
    /// Returns a [`CoinmarketcapError`] if `request.symbols` is empty, the HTTP request fails,
    /// the payload cannot be deserialized, or CoinMarketCap reports a non-zero `status.error_code`.
    pub fn quotes_latest(
        &self,
        request: &CoinmarketcapQuotesRequest,
    ) -> Result<CoinmarketcapQuotesResponse, CoinmarketcapError> {
        if request.symbols.is_empty() {
            return Err(CoinmarketcapError::MissingSymbols);
        }

        let mut url = self.base_url.join("v2/cryptocurrency/quotes/latest")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("symbol", &request.symbols.join(","));
            if !request.convert.is_empty() {
                query.append_pair("convert", &request.convert.join(","));
            }
        }

        let response = self
            .http
            .get(url)
            .header(Self::API_KEY_HEADER, &self.api_key)
            .header(ACCEPT, "application/json")
            .send()?;

        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            if let Ok(envelope) = serde_json::from_str::<CoinmarketcapErrorEnvelope>(&body)
                && envelope.status.error_code != 0
            {
                return Err(CoinmarketcapError::Server {
                    error_code: envelope.status.error_code,
                    error_message: envelope
                        .status
                        .error_message
                        .unwrap_or_else(|| "unknown CoinMarketCap error".to_string()),
                });
            }
            return Err(CoinmarketcapError::UnexpectedStatus {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: CoinmarketcapQuotesResponse = match serde_json::from_str(&body) {
            Ok(resp) => resp,
            Err(json_err) => {
                if let Ok(envelope) = serde_json::from_str::<CoinmarketcapErrorEnvelope>(&body)
                    && envelope.status.error_code != 0
                {
                    return Err(CoinmarketcapError::Server {
                        error_code: envelope.status.error_code,
                        error_message: envelope
                            .status
                            .error_message
                            .unwrap_or_else(|| "unknown CoinMarketCap error".to_string()),
                    });
                }
                return Err(CoinmarketcapError::Json(json_err));
            }
        };

        if parsed.status.error_code != 0 {
            return Err(CoinmarketcapError::Server {
                error_code: parsed.status.error_code,
                error_message: parsed
                    .status
                    .error_message
                    .unwrap_or_else(|| "unknown CoinMarketCap error".to_string()),
            });
        }

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use mockito::{Matcher, Server};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[googletest::test]
    fn quotes_latest_preserves_multi_currency_quotes_and_null_prices()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let payload = r#"{
            "status": {
                "timestamp": "2025-04-04T12:00:00.000Z",
                "error_code": 0,
                "error_message": null,
                "elapsed": 14,
                "credit_count": 2,
                "notice": null
            },
            "data": {
                "BTC": [
                    {
                        "id": 1,
                        "name": "Bitcoin",
                        "symbol": "BTC",
                        "slug": "bitcoin",
                        "num_market_pairs": 11000,
                        "date_added": "2010-07-13T00:00:00.000Z",
                        "tags": [
                            {
                                "slug": "mineable",
                                "name": "Mineable",
                                "category": "OTHERS"
                            }
                        ],
                        "max_supply": 21000000,
                        "circulating_supply": 19840000,
                        "total_supply": 19840000,
                        "is_active": 1,
                        "infinite_supply": false,
                        "cmc_rank": 1,
                        "is_fiat": 0,
                        "last_updated": "2025-04-04T11:59:00.000Z",
                        "quote": {
                            "USD": {
                                "price": 84250.5,
                                "volume_24h": 28500000000.0,
                                "percent_change_24h": 1.75,
                                "market_cap": 1671529920000.0,
                                "last_updated": "2025-04-04T11:59:00.000Z"
                            },
                            "EUR": {
                                "price": 77100.25,
                                "volume_24h": 26100000000.0,
                                "percent_change_24h": 1.62,
                                "market_cap": 1529668960000.0,
                                "last_updated": "2025-04-04T11:59:00.000Z"
                            }
                        }
                    },
                    {
                        "id": 999999,
                        "name": "Untracked Bitcoin Fork",
                        "symbol": "BTC",
                        "slug": "untracked-bitcoin-fork",
                        "is_active": 0,
                        "quote": {
                            "USD": {
                                "price": null,
                                "last_updated": "2025-04-04T11:59:00.000Z"
                            }
                        }
                    }
                ]
            }
        }"#;

        let mock = server
            .mock("GET", "/v2/cryptocurrency/quotes/latest")
            .match_header("X-CMC_PRO_API_KEY", "cmc-secret")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("symbol".into(), "BTC".into()),
                Matcher::UrlEncoded("convert".into(), "USD,EUR".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(payload)
            .create();

        let client = CoinmarketcapClient::with_base_url("cmc-secret", Url::parse(&server.url())?);
        let request = CoinmarketcapQuotesRequest::new(["BTC"]).with_convert(["USD", "EUR"]);
        let response = client.quotes_latest(&request)?;
        mock.assert();

        assert_that!(response.status.error_code, eq(0));
        assert_that!(response.status.credit_count, eq(2));

        let btc_assets = response.data.get("BTC").ok_or("missing BTC entry")?;
        assert_that!(btc_assets.len(), eq(2));

        let primary = btc_assets.first().ok_or("missing primary BTC")?;
        assert_that!(primary.id, eq(1));
        assert_that!(&primary.name, eq("Bitcoin"));
        assert_that!(primary.tags.len(), eq(1));

        let usd_quote = primary.quote.get("USD").ok_or("missing USD quote")?;
        assert_that!(usd_quote.price, some(eq(Decimal::from_str("84250.5")?)));

        let eur_quote = primary.quote.get("EUR").ok_or("missing EUR quote")?;
        assert_that!(eur_quote.price, some(eq(Decimal::from_str("77100.25")?)));

        let untracked = btc_assets.get(1).ok_or("missing second BTC entry")?;
        let untracked_usd = untracked.quote.get("USD").ok_or("missing untracked USD")?;
        assert_that!(untracked_usd.price, none());
        Ok(())
    }

    #[googletest::test]
    fn quotes_latest_rejects_empty_symbols() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let client = CoinmarketcapClient::new("cmc-key")?;
        let request = CoinmarketcapQuotesRequest::default();
        let result = client.quotes_latest(&request);
        assert_that!(result, err(pat!(CoinmarketcapError::MissingSymbols)));
        Ok(())
    }

    #[googletest::test]
    fn quotes_latest_returns_server_error_on_api_failure()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut server = Server::new();
        let err_payload = r#"{
            "status": {
                "timestamp": "2025-04-04T12:00:00.000Z",
                "error_code": 1002,
                "error_message": "API key missing.",
                "elapsed": 0,
                "credit_count": 0
            }
        }"#;

        let mock = server
            .mock("GET", "/v2/cryptocurrency/quotes/latest")
            .match_query(Matcher::Any)
            .with_status(401)
            .with_body(err_payload)
            .create();

        let client = CoinmarketcapClient::with_base_url("bad-key", Url::parse(&server.url())?);
        let request = CoinmarketcapQuotesRequest::new(["BTC"]);
        let result = client.quotes_latest(&request);
        mock.assert();

        assert_that!(
            result,
            err(pat!(CoinmarketcapError::Server {
                error_code: eq(&1002_i64),
                error_message: eq("API key missing."),
            }))
        );
        Ok(())
    }
}
