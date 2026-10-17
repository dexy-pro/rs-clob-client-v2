#![cfg(feature = "data")]
//! Data API v2 routes. Fixtures under `tests/fixtures/data_v2_*.json` are trimmed responses
//! recorded from `https://data-api.polymarket.com` on 2026-10-17, except where a test says the
//! row is built from the documented schema.

use std::str::FromStr as _;

use httpmock::{Method::GET, MockServer};
use polymarket_client_sdk_v2::data::Client;
use polymarket_client_sdk_v2::data::types::Side;
use polymarket_client_sdk_v2::data::types::v2::{
    PositionStatus, PositionsRequest, TokenModule, TokensRequest, TradesRequest, ValueRequest,
};
use polymarket_client_sdk_v2::types::{ConditionId, U256, address};
use reqwest::StatusCode;
use rust_decimal_macros::dec;

const LEADER: &str = "0xf753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a";
const CTF_CONDITION: &str = "0xfd9082cd86853a3c7707ccac8a8a1f36ed929291abcb97ba2c91daa4ba3eb1e0";
const COMBO_CONDITION: &str = "0x03eebcfb2ee36d4fe02ae9b8503274e42a0000000000000000000000000000";

fn condition(s: &str) -> ConditionId {
    s.parse().expect("condition id")
}

fn fixture(name: &str) -> serde_json::Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let body = std::fs::read_to_string(&path).expect("fixture file");
    serde_json::from_str(&body).expect("fixture json")
}

#[tokio::test]
async fn trades_page_keeps_ctf_and_bytes31_rows_and_its_cursor() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/trades")
            .query_param("user", LEADER)
            .query_param("taker_only", "false")
            .query_param("limit", "500");
        then.status(StatusCode::OK)
            .json_body(fixture("data_v2_trades.json"));
    });

    let request = TradesRequest::builder()
        .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
        .taker_only(false)
        .limit(500)
        .build();
    let page = client.trades_v2(&request).await?;

    mock.assert();
    assert_eq!(page.data.len(), 2, "one bytes31 row must not sink the page");
    let ctf = &page.data[0];
    assert_eq!(ctf.condition_id, condition(CTF_CONDITION));
    assert_eq!(ctf.side, Side::Buy);
    assert_eq!(ctf.size, dec!(6.172836));
    assert_eq!(ctf.price, dec!(0.8099999741));
    assert_eq!(ctf.outcome_index, 999);
    assert_eq!(ctf.timestamp, 1_792_201_815);
    let combo = &page.data[1];
    assert_eq!(combo.condition_id, condition(COMBO_CONDITION));
    assert_eq!(
        combo.condition_id.to_string(),
        COMBO_CONDITION,
        "the 31-byte form round-trips"
    );
    assert_eq!(
        ConditionId::from_v2_position_id(combo.token_id),
        (condition(COMBO_CONDITION), 0),
        "the row's token id encodes its condition"
    );
    assert!(page.pagination.has_more);
    assert!(page.pagination.next_cursor.is_some());
    Ok(())
}

#[tokio::test]
async fn trades_follow_the_cursor_until_it_is_null() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let last = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/trades")
            .query_param("user", LEADER)
            .query_param("cursor", "next-page");
        then.status(StatusCode::OK).json_body(serde_json::json!({
            "data": [],
            "pagination": {"limit": 500, "offset": 500, "has_more": false, "next_cursor": null}
        }));
    });

    let mut request = TradesRequest::builder()
        .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
        .build();
    request.cursor = Some("next-page".to_owned());
    let page = client.trades_v2(&request).await?;

    last.assert();
    assert!(page.data.is_empty());
    assert!(!page.pagination.has_more);
    assert_eq!(page.pagination.next_cursor, None);
    Ok(())
}

#[tokio::test]
async fn positions_page_decodes_the_v2_row() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/positions")
            .query_param("user", LEADER)
            .query_param("status", "REDEEMABLE");
        then.status(StatusCode::OK)
            .json_body(fixture("data_v2_positions.json"));
    });

    let request = PositionsRequest::builder()
        .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
        .status(PositionStatus::Redeemable)
        .build();
    let page = client.positions_v2(&request).await?;

    mock.assert();
    let row = &page.data[0];
    assert_eq!(
        row.token_id,
        U256::from_str(
            "115471406278457772646165365718068474644001192645386099926191854205200987280962"
        )?
    );
    assert_eq!(
        row.condition_id,
        condition("0x83577f142332a7a90169ab35db5ec039abb2cb0429120bf863db75e04d56e428")
    );
    assert_eq!(row.current_size, dec!(10));
    assert_eq!(row.entry_cost_usdc, dec!(5));
    assert_eq!(row.total_cost_usdc, dec!(5.125));
    assert_eq!(row.current_value, dec!(9.995));
    assert_eq!(row.status, PositionStatus::Open);
    assert!(!row.redeemable);
    assert_eq!(row.outcome_index, 1);
    assert!(row.opposite_token_id.is_some());
    assert_eq!(row.end_date.to_string(), "2026-10-16");
    Ok(())
}

#[tokio::test]
async fn value_unwraps_the_data_envelope() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/value")
            .query_param("user", LEADER);
        then.status(StatusCode::OK)
            .json_body(fixture("data_v2_value.json"));
    });

    let request = ValueRequest::builder()
        .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
        .build();
    let value = client.value_v2(&request).await?;

    mock.assert();
    assert_eq!(value.value, dec!(149.4846));
    assert_eq!(
        value.proxy_wallet,
        address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a")
    );
    Ok(())
}

#[tokio::test]
async fn tokens_distinguish_ctf_v2_and_unknown_ids() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let ctf_id = "89264922214887084482987024262661936734253296384243115225704485778301560272625";
    let ctf = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/tokens")
            .query_param("token_id", ctf_id);
        then.status(StatusCode::OK)
            .json_body(fixture("data_v2_tokens_ctf.json"));
    });
    // Built from the documented schema: no live V2 single market existed on 2026-10-17.
    let v2_condition = format!("{COMBO_CONDITION}00");
    let v2_id = "1778752447364788174379468102036537031873325219828321400402142166420920729601";
    let v2 = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/tokens")
            .query_param("token_id", v2_id);
        then.status(StatusCode::OK)
            .json_body(serde_json::json!({"data": [{
                "token_id": v2_id,
                "condition_id": v2_condition,
                "structural_condition_id": v2_condition,
                "module": "binary",
                "outcome_index": 1,
                "clob_index": null,
                "outcome": "No",
                "opposite_token_id": null,
                "resolved": false,
                "final_price": null,
                "title": null,
                "market_slug": null,
                "closed": null,
                "neg_risk": null,
                "neg_risk_market_id": null,
                "question_index": null,
                "event_id": null,
                "event_slug": null
            }]}));
    });
    // Live: an id the API does not know (here a combo token) is an empty list.
    let unknown = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/tokens")
            .query_param("token_id", "7");
        then.status(StatusCode::OK)
            .json_body(serde_json::json!({"data": []}));
    });

    let ctf_rows = client
        .tokens(&TokensRequest::TokenIds(vec![U256::from_str(ctf_id)?]))
        .await?;
    let v2_rows = client
        .tokens(&TokensRequest::TokenIds(vec![U256::from_str(v2_id)?]))
        .await?;
    let unknown_rows = client
        .tokens(&TokensRequest::TokenIds(vec![U256::from(7_u8)]))
        .await?;

    ctf.assert();
    v2.assert();
    unknown.assert();
    assert_eq!(ctf_rows[0].module, TokenModule::V1Ctf);
    assert_eq!(ctf_rows[0].structural_condition_id, None);
    assert_eq!(ctf_rows[0].outcome_index, 1);
    assert_eq!(ctf_rows[0].condition_id, condition(CTF_CONDITION));
    assert_eq!(v2_rows[0].module, TokenModule::Binary);
    let structural = v2_rows[0]
        .structural_condition_id
        .expect("V2 rows carry a structural id");
    assert_eq!(
        structural.to_bytes31(),
        condition(COMBO_CONDITION).to_bytes31(),
        "the padded structural id narrows to the on-chain bytes31"
    );
    assert!(unknown_rows.is_empty());
    Ok(())
}

#[tokio::test]
async fn malformed_condition_in_a_row_is_an_error_not_a_guess() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let mut page = fixture("data_v2_trades.json");
    page["data"][0]["condition_id"] = serde_json::json!("0x1234");
    server.mock(|when, then| {
        when.method(GET).path("/v2/trades");
        then.status(StatusCode::OK).json_body(page);
    });

    let result = client.trades_v2(&TradesRequest::default()).await;

    assert!(
        result.is_err(),
        "a 2-byte condition id is not silently accepted"
    );
    Ok(())
}

#[tokio::test]
async fn condition_filter_is_comma_separated_at_each_width() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = Client::new(&server.base_url())?;
    let joined = format!("{CTF_CONDITION},{COMBO_CONDITION}");
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/v2/positions")
            .query_param("user", LEADER)
            .query_param("condition", &joined);
        then.status(StatusCode::OK).json_body(serde_json::json!({
            "data": [],
            "pagination": {"limit": 100, "offset": 0, "has_more": false, "next_cursor": null}
        }));
    });

    let request = PositionsRequest::builder()
        .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
        .condition(vec![condition(CTF_CONDITION), condition(COMBO_CONDITION)])
        .build();
    client.positions_v2(&request).await?;

    mock.assert();
    Ok(())
}
