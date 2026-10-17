//! Request and response types for the Data API v2 routes (`/v2/...`).
//!
//! Data API v1 is retired on 2026-10-24 (docs `/migrate/data-api-v1-to-v2`). The v2 routes
//! share one contract: every response wraps its payload in `data`, paginated routes add a
//! `pagination` object and page with an opaque cursor, rows are `snake_case`, and markets are
//! selected with `condition` (at most 20 condition ids).
//!
//! To read the next page, send the same anchor (`user` or `condition`) with
//! `cursor = page.pagination.next_cursor`, until `next_cursor` is `None`.

#![allow(
    clippy::module_name_repetitions,
    reason = "Request suffix is intentional for clarity"
)]

use bon::Builder;
use chrono::NaiveDate;
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::{
    DisplayFromStr, NoneAsEmptyString, StringWithSeparator, formats::CommaSeparator, serde_as,
    skip_serializing_none,
};

use super::{FilterType, Side};
use crate::types::{Address, B256, ConditionId, Decimal, U256};

/// The lifecycle filter of `/v2/positions` and each row's own state.
///
/// `Open` is the superset: it includes settled-but-unredeemed positions, which `Redeemable`
/// narrows to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PositionStatus {
    /// Held positions, resolved-but-unredeemed winners included.
    Open,
    /// Held positions whose market resolved.
    Redeemable,
    /// Held losing positions of resolved markets.
    RedeemableLost,
    /// Live complementary pairs the wallet can merge.
    Mergeable,
    /// Exited positions.
    Closed,
    /// Unknown status from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}

/// The contract family that issued a token (`/v2/tokens` `module`).
///
/// `V1Ctf` is a `ConditionalTokens` (CTF) token; every other value is a Polymarket Protocol V2
/// module held in `PositionManager`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[non_exhaustive]
pub enum TokenModule {
    /// A `ConditionalTokens` (CTF) market token.
    #[serde(rename = "v1_ctf")]
    #[strum(serialize = "v1_ctf")]
    V1Ctf,
    /// A Protocol V2 binary market position.
    #[serde(rename = "binary")]
    #[strum(serialize = "binary")]
    Binary,
    /// A Protocol V2 neg-risk market position.
    #[serde(rename = "neg_risk")]
    #[strum(serialize = "neg_risk")]
    NegRisk,
    /// A Protocol V2 combo position.
    #[serde(rename = "combo")]
    #[strum(serialize = "combo")]
    Combo,
    /// Unknown module from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}

/// One `{ data, pagination }` page of a paginated v2 route.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Page<T> {
    /// The page's rows.
    pub data: Vec<T>,
    /// Where the walk stands.
    pub pagination: Pagination,
}

/// The `pagination` object of a v2 page.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Pagination {
    /// Page size this page was served with.
    pub limit: u32,
    /// Running item offset, for display only; the cursor drives the seek.
    pub offset: u32,
    /// Whether another page exists.
    pub has_more: bool,
    /// Cursor for the next page; `None` on the last page.
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// The `{ data }` envelope of the non-paginated v2 routes.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    pub(crate) data: T,
}

/// Request for `GET /v2/positions`. At least one of `user` and `condition` is required.
///
/// # Example
///
/// ```
/// use polymarket_client_sdk_v2::data::types::v2::{PositionStatus, PositionsRequest};
/// use polymarket_client_sdk_v2::types::address;
///
/// let request = PositionsRequest::builder()
///     .user(address!("56687bf447db6ffa42ffe2204a05edaa20f55839"))
///     .status(PositionStatus::Redeemable)
///     .limit(500)
///     .build();
/// ```
#[serde_as]
#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Default, Builder, Serialize)]
#[builder(on(String, into))]
pub struct PositionsRequest {
    /// The wallet to anchor on.
    pub user: Option<Address>,
    /// Condition ids to narrow to (at most 20).
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, ConditionId>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub condition: Vec<ConditionId>,
    /// Lifecycle filter; the server defaults to [`PositionStatus::Open`].
    pub status: Option<PositionStatus>,
    /// Unit of `filter_amount`; the server defaults to tokens.
    pub filter_type: Option<FilterType>,
    /// Minimum current holding (tokens) or value (cash); the server default is a 0.1-share
    /// dust floor.
    pub filter_amount: Option<Decimal>,
    /// First-page size (at most 1000); ignored when `cursor` is set.
    pub limit: Option<u32>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<String>,
}

/// Request for `GET /v2/trades`.
#[serde_as]
#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Default, Builder, Serialize)]
#[builder(on(String, into))]
pub struct TradesRequest {
    /// Wallet whose trades to list; omit for the market or global feed.
    pub user: Option<Address>,
    /// Condition ids to narrow to (at most 20).
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, ConditionId>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub condition: Vec<ConditionId>,
    /// Only BUY or only SELL rows.
    pub side: Option<Side>,
    /// The server defaults to `true` (each fill once, on its taker side); `false` adds the
    /// maker rows.
    pub taker_only: Option<bool>,
    /// Unit of `filter_amount`; the server defaults to tokens.
    pub filter_type: Option<FilterType>,
    /// Minimum trade size; the server defaults to 0.01.
    pub filter_amount: Option<Decimal>,
    /// Inclusive lower bound on the block timestamp (epoch seconds; `user` shape only).
    pub start: Option<i64>,
    /// Inclusive upper bound on the block timestamp (epoch seconds; `user` shape only).
    pub end: Option<i64>,
    /// First-page size (at most 1000); ignored when `cursor` is set.
    pub limit: Option<u32>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<String>,
}

/// Request for `GET /v2/value`.
#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, Builder, Serialize)]
pub struct ValueRequest {
    /// The wallet whose portfolio value to return.
    pub user: Address,
    /// Condition ids to scope the single-market term to (at most 20); empty for the whole
    /// portfolio.
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, ConditionId>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub condition: Vec<ConditionId>,
}

/// Request for `GET /v2/tokens`: exactly one selector.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokensRequest {
    /// Up to 50 token ids (CTF token ids or V2 position ids).
    TokenIds(Vec<U256>),
    /// Up to 10 condition selectors: CTF or V2 condition ids, neg-risk market ids, or V2
    /// neg-risk event ids.
    Conditions(Vec<ConditionId>),
}

impl Serialize for TokensRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        fn join<T: ToString>(items: &[T]) -> String {
            items
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        }
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::TokenIds(ids) => map.serialize_entry("token_id", &join(ids))?,
            Self::Conditions(ids) => map.serialize_entry("condition", &join(ids))?,
        }
        map.end()
    }
}

/// One `/v2/positions` row: a holding in a single outcome token.
#[serde_as]
#[expect(
    clippy::struct_excessive_bools,
    reason = "The current API has these fields, so we have to capture this"
)]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Position {
    /// Wallet holding the position.
    pub proxy_wallet: Address,
    /// The outcome token id (a CTF token id or a V2 position id).
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    /// The market's condition id.
    pub condition_id: ConditionId,
    /// Current holding in shares (about zero on `CLOSED` rows).
    pub current_size: Decimal,
    /// Weighted-average entry price per share.
    pub avg_price: Decimal,
    /// Fee-exclusive entry basis.
    pub entry_cost_usdc: Decimal,
    /// Attributed BUY fees (disclosure only; already excluded from `entry_cost_usdc`).
    pub entry_fees_usdc: Decimal,
    /// Fee-inclusive basis (`entry_cost_usdc + entry_fees_usdc`).
    pub total_cost_usdc: Decimal,
    /// Live mark per share.
    pub current_price: Decimal,
    /// `current_size * current_price`.
    pub current_value: Decimal,
    /// Lifetime bought shares, not the current balance.
    pub total_size: Decimal,
    /// Cumulative realized profit and loss.
    pub realized_pnl: Decimal,
    /// `current_value - entry_cost_usdc`.
    pub unrealized_pnl: Decimal,
    /// `realized_pnl + unrealized_pnl`.
    pub total_pnl: Decimal,
    /// Unrealized profit and loss over the fee-exclusive basis, in percent.
    pub percent_pnl: Decimal,
    /// Compatibility percentage; not realized profit and loss over a basis.
    pub percent_realized_pnl: Decimal,
    /// The row's own state, which can be narrower than the requested status.
    pub status: PositionStatus,
    /// Resolved and still held (losing sides included).
    pub redeemable: bool,
    /// The wallet also holds the opposite outcome.
    pub mergeable: bool,
    /// The market belongs to a neg-risk group.
    pub negative_risk: bool,
    /// The market is archived.
    pub archived: bool,
    /// Market question.
    pub title: String,
    /// Market slug.
    pub slug: String,
    /// Market icon URL.
    pub icon: String,
    /// Gamma event id of the parent event.
    pub event_id: String,
    /// Parent event slug.
    pub event_slug: String,
    /// Label of the held outcome.
    pub outcome: String,
    /// Index of the held outcome; `999` when the outcome could not be labeled.
    pub outcome_index: i32,
    /// Label of the other outcome.
    pub opposite_outcome: String,
    /// Token id of the other outcome, when there is one.
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub opposite_token_id: Option<U256>,
    /// Market end date (`1970-01-01` when Gamma has none).
    pub end_date: NaiveDate,
    /// Last economics event, epoch seconds; 0 without native state.
    pub last_event_at: i64,
    /// First acquisition, epoch seconds; 0 when none is recorded.
    pub first_entry_at: i64,
    /// Profile display name of the wallet.
    pub name: String,
    /// Profile image URL.
    pub profile_image: String,
    /// Profile verification badge.
    pub verified: bool,
}

/// One `/v2/trades` row.
#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Trade {
    /// Wallet the row belongs to.
    pub proxy_wallet: Address,
    /// `BUY` or `SELL`, from this wallet's side.
    pub side: Side,
    /// Traded outcome token id (a CTF token id or a V2 position id).
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    /// The market's condition id.
    pub condition_id: ConditionId,
    /// Filled shares.
    pub size: Decimal,
    /// Execution price per share.
    pub price: Decimal,
    /// Block timestamp of the fill, epoch seconds.
    pub timestamp: i64,
    /// Market question.
    pub title: String,
    /// Market slug.
    pub slug: String,
    /// Market icon URL.
    pub icon: String,
    /// Parent event slug.
    pub event_slug: String,
    /// Label of the traded outcome.
    pub outcome: String,
    /// Index of the traded outcome; `999` when the outcome could not be labeled.
    pub outcome_index: i32,
    /// Profile display name of the wallet.
    pub name: String,
    /// Generated fallback handle.
    pub pseudonym: String,
    /// Profile bio.
    pub bio: String,
    /// Profile image URL.
    pub profile_image: String,
    /// Resized profile image URL.
    pub profile_image_optimized: String,
    /// Hash of the settling transaction.
    pub transaction_hash: B256,
}

/// The `/v2/value` result: holdings marked to market plus unresolved combos at cost basis.
/// Cash is not included.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PortfolioValue {
    /// The wallet the value was computed for.
    pub proxy_wallet: Address,
    /// Portfolio value in USDC, rounded to 4 decimals.
    pub value: Decimal,
}

/// One `/v2/tokens` row: the reference facts for one token id.
///
/// A token id the API does not know is absent from the response; it is never an error.
#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TokenRef {
    /// The token id (a CTF token id or a V2 position id).
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    /// The market's condition id in its 66-character form, as the other routes carry it.
    pub condition_id: ConditionId,
    /// The on-chain V2 condition id in its 66-character form; `None` for `v1_ctf`. It equals
    /// `condition_id` on a native V2 market and differs on a market carried over to V2.
    #[serde(default)]
    pub structural_condition_id: Option<ConditionId>,
    /// The contract family that issued the token.
    pub module: TokenModule,
    /// On-chain outcome slot of the token within its condition.
    pub outcome_index: i32,
    /// Position in the market's CLOB token list (`v1_ctf` only).
    #[serde(default)]
    pub clob_index: Option<i32>,
    /// Outcome label.
    #[serde(default)]
    pub outcome: Option<String>,
    /// The other token of a two-outcome condition.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub opposite_token_id: Option<U256>,
    /// The token has an on-chain payout.
    pub resolved: bool,
    /// Payout rate once resolved (0 lost, 1 won).
    #[serde(default)]
    pub final_price: Option<Decimal>,
    /// Market question.
    #[serde(default)]
    pub title: Option<String>,
    /// Market slug.
    #[serde(default)]
    pub market_slug: Option<String>,
    /// The market is closed.
    #[serde(default)]
    pub closed: Option<bool>,
    /// The market belongs to a neg-risk structure.
    #[serde(default)]
    pub neg_risk: Option<bool>,
    /// The neg-risk structure's market id.
    #[serde(default)]
    pub neg_risk_market_id: Option<String>,
    /// Index of the market inside its neg-risk structure.
    #[serde(default)]
    pub question_index: Option<i32>,
    /// Lowest event id the market belongs to.
    #[serde(default)]
    pub event_id: Option<i64>,
    /// Slug of that event.
    #[serde(default)]
    pub event_slug: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToQueryParams as _;
    use crate::types::address;

    const V2: &str = "0x03eebcfb2ee36d4fe02ae9b8503274e42a0000000000000000000000000000";
    const CTF: &str = "0xfd9082cd86853a3c7707ccac8a8a1f36ed929291abcb97ba2c91daa4ba3eb1e0";

    fn condition(s: &str) -> ConditionId {
        s.parse().expect("condition id")
    }

    #[test]
    fn positions_request_serializes_the_v2_names() {
        let request = PositionsRequest::builder()
            .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
            .condition(vec![condition(CTF), condition(V2)])
            .status(PositionStatus::RedeemableLost)
            .filter_type(FilterType::Tokens)
            .filter_amount(Decimal::ZERO)
            .limit(500)
            .cursor("abc")
            .build();

        assert_eq!(
            request.query_params(None),
            format!(
                "?user=0xf753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a&condition={CTF}%2C{V2}\
                 &status=REDEEMABLE_LOST&filter_type=TOKENS&filter_amount=0&limit=500&cursor=abc"
            )
        );
        assert_eq!(PositionsRequest::default().query_params(None), "");
    }

    #[test]
    fn trades_request_serializes_the_v2_names() {
        let request = TradesRequest::builder()
            .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
            .side(Side::Sell)
            .taker_only(false)
            .start(1_792_201_000)
            .limit(200)
            .build();

        assert_eq!(
            request.query_params(None),
            "?user=0xf753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a&side=SELL&taker_only=false\
             &start=1792201000&limit=200"
        );
    }

    #[test]
    fn tokens_request_sends_exactly_one_selector() {
        let ids = TokensRequest::TokenIds(vec![U256::from(1_u8), U256::from(22_u8)]);
        let conditions = TokensRequest::Conditions(vec![condition(V2)]);

        assert_eq!(ids.query_params(None), "?token_id=1%2C22");
        assert_eq!(conditions.query_params(None), format!("?condition={V2}"));
    }

    #[test]
    fn value_request_requires_a_user() {
        let request = ValueRequest::builder()
            .user(address!("f753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"))
            .build();

        assert_eq!(
            request.query_params(None),
            "?user=0xf753f261f0c07ef3ec3b9eb15d51cbdaa5340b0a"
        );
    }

    #[test]
    fn token_module_and_status_use_wire_names() {
        let module: TokenModule = serde_json::from_str("\"v1_ctf\"").expect("module");
        let unknown: TokenModule = serde_json::from_str("\"future\"").expect("module");
        let status: PositionStatus = serde_json::from_str("\"REDEEMABLE_LOST\"").expect("status");

        assert_eq!(module, TokenModule::V1Ctf);
        assert_eq!(unknown, TokenModule::Unknown("future".to_owned()));
        assert_eq!(status, PositionStatus::RedeemableLost);
        assert_eq!(TokenModule::NegRisk.to_string(), "neg_risk");
    }
}
