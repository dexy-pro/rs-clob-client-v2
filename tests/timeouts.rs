#![cfg(all(feature = "clob", feature = "gamma", feature = "data"))]
//! A configured timeout bounds each HTTP request of the CLOB, Gamma and Data clients, and the
//! error says it was a timeout.

use std::time::Duration;

use httpmock::{Method::GET, MockServer};
use polymarket_client_sdk_v2::clob::types::request::OrderBookSummaryRequest;
use polymarket_client_sdk_v2::clob::{self, Config};
use polymarket_client_sdk_v2::data::{self, types::v2::TradesRequest};
use polymarket_client_sdk_v2::gamma::{self, types::request::MarketsRequest};
use polymarket_client_sdk_v2::types::U256;
use reqwest::StatusCode;

const TIMEOUT: Duration = Duration::from_millis(100);
const HANG: Duration = Duration::from_secs(5);

fn hanging_server(path: &str) -> MockServer {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path(path);
        then.status(StatusCode::OK).delay(HANG).body("{}");
    });
    server
}

#[tokio::test]
async fn clob_request_gives_up_after_the_configured_timeout() -> anyhow::Result<()> {
    let server = hanging_server("/book");
    let client = clob::Client::new(
        &server.base_url(),
        Config::builder().timeout(TIMEOUT).build(),
    )?;
    let request = OrderBookSummaryRequest::builder()
        .token_id(U256::from(1_u8))
        .build();

    let started = std::time::Instant::now();
    let err = client
        .order_book(&request)
        .await
        .expect_err("a hanging response must time out");

    assert!(err.is_timeout(), "the error is a timeout: {err}");
    assert!(started.elapsed() < HANG, "it did not wait for the response");
    Ok(())
}

#[tokio::test]
async fn gamma_request_gives_up_after_the_configured_timeout() -> anyhow::Result<()> {
    let server = hanging_server("/markets");
    let client = gamma::Client::with_timeout(&server.base_url(), TIMEOUT)?;

    let err = client
        .markets(&MarketsRequest::builder().build())
        .await
        .expect_err("a hanging response must time out");

    assert!(err.is_timeout(), "the error is a timeout: {err}");
    Ok(())
}

#[tokio::test]
async fn data_request_gives_up_after_the_configured_timeout() -> anyhow::Result<()> {
    let server = hanging_server("/v2/trades");
    let client = data::Client::with_timeout(&server.base_url(), TIMEOUT)?;

    let err = client
        .trades_v2(&TradesRequest::default())
        .await
        .expect_err("a hanging response must time out");

    assert!(err.is_timeout(), "the error is a timeout: {err}");
    Ok(())
}

#[tokio::test]
async fn a_status_error_is_not_a_timeout() -> anyhow::Result<()> {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/v2/trades");
        then.status(StatusCode::SERVICE_UNAVAILABLE).body("{}");
    });
    let client = data::Client::with_timeout(&server.base_url(), TIMEOUT)?;

    let err = client
        .trades_v2(&TradesRequest::default())
        .await
        .expect_err("503");

    assert!(!err.is_timeout(), "a server answer is not a timeout: {err}");
    Ok(())
}
