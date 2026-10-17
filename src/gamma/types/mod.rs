//! Types for the Polymarket Gamma API.
//!
//! This module contains all types used by the Gamma API client, organized into:
//!
//! - **Common types**: Shared data structures used across requests and responses,
//!   as well as enums for filtering and categorization.
//!
//! - **Request types**: Builder-pattern structs for each API endpoint
//!   (e.g., [`request::EventsRequest`], [`request::MarketsRequest`]).
//!
//! - **Response types**: Structs representing API responses
//!   (e.g., [`response::Event`], [`response::Market`], [`response::Tag`]).
//!
//! # Request Building
//!
//! All request types use the builder pattern via the [`bon`](https://docs.rs/bon) crate:
//!
//! ```
//! use polymarket_client_sdk_v2::gamma::types::request::{EventsRequest, MarketsRequest};
//!
//! // Simple request with defaults
//! let events = EventsRequest::builder().build();
//!
//! // Request with filters
//! let markets = MarketsRequest::builder()
//!     .limit(10)
//!     .closed(false)
//!     .build();
//! ```

use serde::{Deserialize, Serialize};

pub mod request;
pub mod response;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[non_exhaustive]
pub enum RelatedTagsStatus {
    Active,
    Closed,
    All,
    /// Unknown status from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[non_exhaustive]
pub enum ParentEntityType {
    Event,
    Series,
    #[serde(rename = "market")]
    #[strum(serialize = "market")]
    Market,
    /// Unknown entity type from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}

/// The protocol a Gamma market trades on, from its `version` field.
///
/// The V2 API migration guide says to choose the outcome ids by `version` even when both id
/// arrays are present: `v2` uses `positionIds`, `v1` uses `clobTokenIds`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[non_exhaustive]
pub enum MarketVersion {
    /// A CTF market: outcome ids are `clobTokenIds`.
    V1,
    /// A Polymarket Protocol V2 market: outcome ids are `positionIds`.
    V2,
    /// Unknown version from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}

/// Resolution state of a Protocol V2 market (`resolutionStatus`). V1 markets report UMA
/// resolution through `umaResolutionStatus` instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[non_exhaustive]
pub enum ResolutionStatus {
    Inactive,
    Active,
    Resolved,
    /// Unknown status from the API (captures the raw value for debugging).
    #[serde(untagged)]
    Unknown(String),
}
