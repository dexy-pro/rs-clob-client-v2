#![cfg(all(feature = "clob", feature = "gamma"))]
//! Protocol V2 identity on the Gamma and CLOB types Dexy reads: the market `version` selects
//! the outcome ids, and condition ids keep their 31- or 32-byte width.
//!
//! No V2 single market was live on 2026-10-17 (100 of 100 sampled open Gamma markets were
//! `v1`), so the V2 rows here follow the documented schema (docs
//! `/migrate/polymarket-v2/api-integrations`); the CTF rows are recorded live responses.

use std::str::FromStr as _;

use httpmock::{Method::GET, MockServer};
use polymarket_client_sdk_v2::clob::types::response::{
    MarketByTokenResponse, OpenOrderResponse, OrderBookSummaryResponse, TradeResponse,
};
use polymarket_client_sdk_v2::clob::{self, Config};
use polymarket_client_sdk_v2::gamma::types::request::MarketByIdRequest;
use polymarket_client_sdk_v2::gamma::types::{MarketVersion, ResolutionStatus};
use polymarket_client_sdk_v2::gamma::{self};
use polymarket_client_sdk_v2::types::{ConditionId, U256};
use reqwest::StatusCode;
use serde_json::json;

const CTF_CONDITION: &str = "0xfd9082cd86853a3c7707ccac8a8a1f36ed929291abcb97ba2c91daa4ba3eb1e0";
const V2_CONDITION: &str = "0x03eebcfb2ee36d4fe02ae9b8503274e42a0000000000000000000000000000";

fn condition(s: &str) -> ConditionId {
    s.parse().expect("condition id")
}

fn gamma_market(version: &str, extra: &serde_json::Value) -> serde_json::Value {
    let mut market = json!({
        "id": "5792382",
        "question": "Will it happen?",
        "conditionId": format!("{V2_CONDITION}00"),
        "slug": "will-it-happen",
        "outcomes": "[\"Yes\", \"No\"]",
        "outcomePrices": "[\"0.4\", \"0.6\"]",
        "clobTokenIds": "[\"11\", \"22\"]",
        "version": version,
        "active": true,
        "closed": false
    });
    for (key, value) in extra.as_object().expect("object") {
        market[key] = value.clone();
    }
    market
}

#[tokio::test]
async fn gamma_v2_market_carries_version_position_ids_and_resolution() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = gamma::Client::new(&server.base_url())?;
    let position_yes =
        "1778752447364788174379468102036537031873325219828321400402142166420920729600";
    let position_no =
        "1778752447364788174379468102036537031873325219828321400402142166420920729601";
    server.mock(|when, then| {
        when.method(GET).path("/markets/5792382");
        then.status(StatusCode::OK).json_body(gamma_market(
            "v2",
            &json!({"positionIds": [position_yes, position_no], "resolutionStatus": "resolved"}),
        ));
    });

    let market = client
        .market_by_id(&MarketByIdRequest::builder().id("5792382").build())
        .await?;

    assert_eq!(market.version, Some(MarketVersion::V2));
    assert_eq!(
        market.position_ids,
        Some(vec![
            U256::from_str(position_yes)?,
            U256::from_str(position_no)?
        ]),
        "positionIds is a plain array of decimal strings"
    );
    assert_eq!(
        market.clob_token_ids,
        Some(vec![U256::from(11_u8), U256::from(22_u8)]),
        "both arrays survive; the caller picks by version"
    );
    assert_eq!(market.resolution_status, Some(ResolutionStatus::Resolved));
    let condition_id = market.condition_id.expect("condition id");
    assert_eq!(
        condition_id.to_bytes31(),
        condition(V2_CONDITION).to_bytes31(),
        "the padded 66-character V2 condition narrows to its bytes31"
    );
    Ok(())
}

#[tokio::test]
async fn gamma_v1_and_unknown_versions_are_explicit() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = gamma::Client::new(&server.base_url())?;
    server.mock(|when, then| {
        when.method(GET).path("/markets/1");
        then.status(StatusCode::OK).json_body(gamma_market(
            "v1",
            &json!({"id": "1", "conditionId": CTF_CONDITION, "positionIds": null}),
        ));
    });
    server.mock(|when, then| {
        when.method(GET).path("/markets/2");
        then.status(StatusCode::OK)
            .json_body(gamma_market("v3", &json!({"id": "2"})));
    });
    server.mock(|when, then| {
        when.method(GET).path("/markets/3");
        then.status(StatusCode::OK)
            .json_body(json!({"id": "3", "conditionId": CTF_CONDITION}));
    });

    let request = |id: &str| MarketByIdRequest::builder().id(id).build();
    let v1 = client.market_by_id(&request("1")).await?;
    let future = client.market_by_id(&request("2")).await?;
    let missing = client.market_by_id(&request("3")).await?;

    assert_eq!(v1.version, Some(MarketVersion::V1));
    assert_eq!(v1.position_ids, None);
    assert_eq!(v1.condition_id, Some(condition(CTF_CONDITION)));
    assert_eq!(
        future.version,
        Some(MarketVersion::Unknown("v3".to_owned()))
    );
    assert_eq!(missing.version, None, "a missing version stays missing");
    Ok(())
}

#[test]
fn gamma_trading_ids_must_be_decimal() {
    let decode = |extra: serde_json::Value| -> Result<gamma::types::response::Market, _> {
        serde_json::from_value(gamma_market("v2", &extra))
    };
    let overflow = format!("1{}", "0".repeat(78));

    for bad in ["0x10", "+1", "-1", " 1", "1 ", "", "1e3", overflow.as_str()] {
        assert!(
            decode(json!({"positionIds": [bad]})).is_err(),
            "positionIds {bad:?} must be rejected"
        );
        assert!(
            decode(json!({"clobTokenIds": serde_json::to_string(&[bad]).unwrap()})).is_err(),
            "clobTokenIds {bad:?} must be rejected"
        );
    }
    let padded = decode(json!({"positionIds": ["007"], "clobTokenIds": "[\"0042\"]"})).unwrap();
    assert_eq!(padded.position_ids, Some(vec![U256::from(7_u8)]));
    assert_eq!(padded.clob_token_ids, Some(vec![U256::from(42_u8)]));
    let max = U256::MAX.to_string();
    let widest = decode(json!({"positionIds": [max]})).unwrap();
    assert_eq!(widest.position_ids, Some(vec![U256::MAX]));
}

#[tokio::test]
async fn clob_market_by_token_and_book_keep_the_condition_width() -> anyhow::Result<()> {
    let server = MockServer::start();
    let client = clob::Client::new(&server.base_url(), Config::default())?;
    let token = "89264922214887084482987024262661936734253296384243115225704485778301560272625";
    // Live response, 2026-10-17.
    server.mock(|when, then| {
        when.method(GET).path(format!("/markets-by-token/{token}"));
        then.status(StatusCode::OK).json_body(json!({
            "condition_id": CTF_CONDITION,
            "primary_token_id": "4770630589345892248563857692117303241871557348980709371172099395272142547480",
            "secondary_token_id": token
        }));
    });

    let market = client.market_by_token(U256::from_str(token)?).await?;

    assert_eq!(market.condition_id, condition(CTF_CONDITION));
    let book: OrderBookSummaryResponse = serde_json::from_value(json!({
        "market": format!("{V2_CONDITION}00"),
        "asset_id": "1778752447364788174379468102036537031873325219828321400402142166420920729600",
        "timestamp": "1792203330352",
        "hash": "15aa2aac2b7814ed6bb7f493ab15148e47cd8eb5",
        "bids": [{"price": "0.01", "size": "211323"}],
        "asks": [],
        "min_order_size": "5",
        "tick_size": "0.01",
        "neg_risk": false,
        "last_trade_price": "0.5"
    }))?;
    assert_eq!(book.market.to_string(), format!("{V2_CONDITION}00"));
    let by_token: MarketByTokenResponse = serde_json::from_value(json!({
        "condition_id": V2_CONDITION,
        "primary_token_id": "1",
        "secondary_token_id": "2"
    }))?;
    assert_eq!(by_token.condition_id, condition(V2_CONDITION));
    Ok(())
}

#[test]
fn clob_trades_and_orders_with_a_bytes31_market_decode() -> anyhow::Result<()> {
    let trade: TradeResponse = serde_json::from_value(json!({
        "id": "trade-1",
        "taker_order_id": "0xabc",
        "market": V2_CONDITION,
        "asset_id": "1778752447364788174379468102036537031873325219828321400402142166420920729600",
        "side": "BUY",
        "size": "10",
        "fee_rate_bps": "0",
        "price": "0.18",
        "status": "TRADE_STATUS_MINED",
        "match_time": "1792201806",
        "last_update": "1792201806",
        "outcome": "",
        "bucket_index": 0,
        "owner": "f4f247b7-4ac7-ff29-a152-04fda0a8755a",
        "maker_address": "0x0e46fe1506310e628240d9fa6302cceddfada831",
        "maker_orders": [],
        "transaction_hash": "0x4b97b2be7213b7afa6d17d387064a7585172a6631f909d02647d7fab062a0f22",
        "trader_side": "TAKER"
    }))?;
    let order: OpenOrderResponse = serde_json::from_value(json!({
        "id": "0xabcdef",
        "status": "LIVE",
        "owner": "f4f247b7-4ac7-ff29-a152-04fda0a8755a",
        "maker_address": "0x1234567890123456789012345678901234567890",
        "market": format!("{V2_CONDITION}00"),
        "asset_id": "1",
        "side": "SELL",
        "original_size": "100",
        "size_matched": "0",
        "price": "0.5",
        "outcome": "YES",
        "expiration": "0",
        "order_type": "GTC",
        "associate_trades": [],
        "created_at": 1_700_000_000
    }))?;

    assert_eq!(trade.market, condition(V2_CONDITION));
    assert_eq!(
        trade.status,
        polymarket_client_sdk_v2::clob::types::TradeStatusType::Mined,
        "documented TRADE_STATUS_* names map to their states"
    );
    assert_eq!(order.market.to_string(), format!("{V2_CONDITION}00"));
    Ok(())
}
