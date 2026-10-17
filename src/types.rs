//! Re-exported types from external crates for convenience.
//!
//! These types are commonly used in this SDK and are re-exported here
//! so users don't need to add these dependencies to their `Cargo.toml`.

use std::fmt;
use std::str::FromStr;

use alloy::primitives::FixedBytes;
/// Ethereum address type and the [`address!`] macro for compile-time address literals.
/// [`ChainId`] is a type alias for `u64` representing EVM chain IDs.
/// [`Signature`] represents cryptographic signatures for signed orders.
/// [`B256`] is a 256-bit fixed-size byte array type used for hashes.
/// [`U256`] is a 256-bit integer
pub use alloy::primitives::{Address, B256, ChainId, Signature, U256, address, b256};
/// Date and time types for timestamps in API responses and order expiration.
pub use chrono::{DateTime, NaiveDate, Utc};
/// Arbitrary precision decimal type for prices, sizes, and amounts.
pub use rust_decimal::Decimal;
/// Macro for creating [`Decimal`] literals at compile time.
///
/// # Example
/// ```
/// use polymarket_client_sdk_v2::types::dec;
/// let price = dec!(0.55);
/// ```
pub use rust_decimal_macros::dec;
use serde_with::{DeserializeFromStr, SerializeDisplay};

/// The 31-byte on-chain form of a Polymarket Protocol V2 condition id.
pub type Bytes31 = FixedBytes<31>;

/// A Polymarket condition id, kept at the width it was given in.
///
/// CTF markets use 32-byte ids (`0x` + 64 hex digits). Protocol V2 conditions are 31 bytes on
/// chain (`0x` + 62 hex digits, the form combo markets and combo legs carry). The APIs show a
/// V2 market's condition in a 66-character form that appends one zero byte to the 31-byte id.
///
/// Parsing keeps the width it saw, so a leading zero byte is never dropped and a padding byte is
/// never invented. The two forms of one V2 condition are different values here; use
/// [`ConditionId::to_bytes31`] to compare them.
///
/// # Example
///
/// ```
/// use polymarket_client_sdk_v2::types::ConditionId;
///
/// let combo: ConditionId = "0x03eebcfb2ee36d4fe02ae9b8503274e42a0000000000000000000000000000"
///     .parse()
///     .expect("31-byte V2 condition");
/// assert!(combo.as_bytes32().is_none());
/// assert!(combo.to_bytes31().is_some());
/// ```
#[non_exhaustive]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, SerializeDisplay, DeserializeFromStr,
)]
pub enum ConditionId {
    /// A 32-byte id: a CTF condition, or the zero-padded form of a V2 condition.
    Bytes32(B256),
    /// A 31-byte Protocol V2 condition.
    Bytes31(Bytes31),
}

impl ConditionId {
    /// Returns the 32-byte value when this id has that width.
    #[must_use]
    pub const fn as_bytes32(&self) -> Option<B256> {
        match self {
            Self::Bytes32(id) => Some(*id),
            Self::Bytes31(_) => None,
        }
    }

    /// Returns the 31-byte Protocol V2 condition this id names.
    ///
    /// A 31-byte id is returned as is. A 32-byte id narrows only when its final byte is zero (the
    /// padded V2 form); any other 32-byte id is not a V2 condition and returns `None`. Whether a
    /// zero-ending 32-byte id is really a V2 condition is the caller's protocol knowledge.
    #[must_use]
    pub fn to_bytes31(&self) -> Option<Bytes31> {
        match self {
            Self::Bytes31(id) => Some(*id),
            Self::Bytes32(id) => {
                let (head, tail) = id.0.split_at(31);
                (tail == [0]).then(|| Bytes31::from_slice(head))
            }
        }
    }

    /// Splits a Protocol V2 position id into its 31-byte condition and its outcome slot.
    ///
    /// Per the position-management docs, `conditionId = bytes31(positionId >> 8)` and
    /// `outcomeIndex = positionId & 0xff`; all 256 bits are kept, leading zeros included.
    #[must_use]
    pub fn from_v2_position_id(position_id: U256) -> (Self, u8) {
        let bytes = position_id.to_be_bytes::<32>();
        let (condition, outcome) = bytes.split_at(31);
        (Self::Bytes31(Bytes31::from_slice(condition)), outcome[0])
    }
}

impl From<B256> for ConditionId {
    fn from(id: B256) -> Self {
        Self::Bytes32(id)
    }
}

impl From<Bytes31> for ConditionId {
    fn from(id: Bytes31) -> Self {
        Self::Bytes31(id)
    }
}

impl fmt::Display for ConditionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes32(id) => write!(f, "{id}"),
            Self::Bytes31(id) => write!(f, "{id}"),
        }
    }
}

/// Error returned when a string is not a `0x`-prefixed 62- or 64-digit hex condition id.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionIdError {
    /// The rejected input.
    pub input: String,
}

impl fmt::Display for ConditionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid condition id {:?}: expected 0x followed by 62 or 64 hex digits",
            self.input
        )
    }
}

impl std::error::Error for ConditionIdError {}

impl FromStr for ConditionId {
    type Err = ConditionIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ConditionIdError {
            input: s.to_owned(),
        };
        let digits = s.strip_prefix("0x").ok_or_else(invalid)?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        match digits.len() {
            64 => B256::from_str(digits)
                .map(Self::Bytes32)
                .map_err(|_e| invalid()),
            62 => Bytes31::from_str(digits)
                .map(Self::Bytes31)
                .map_err(|_e| invalid()),
            _ => Err(invalid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTF: &str = "0x00c6d1f356d3df1917944cc1b525f2b5184404416b324070d037bd27509558c6";
    const V2: &str = "0x03eebcfb2ee36d4fe02ae9b8503274e42a0000000000000000000000000000";
    const CTF_DIGITS: &str = "00c6d1f356d3df1917944cc1b525f2b5184404416b324070d037bd27509558c6";

    fn parse(s: &str) -> ConditionId {
        s.parse()
            .unwrap_or_else(|e| panic!("{s} should parse: {e}"))
    }

    #[test]
    fn widths_round_trip_with_leading_zeros() {
        let ctf = parse(CTF);
        let v2 = parse(V2);

        assert!(
            matches!(ctf, ConditionId::Bytes32(_)),
            "64 digits are 32 bytes"
        );
        assert!(
            matches!(v2, ConditionId::Bytes31(_)),
            "62 digits are 31 bytes"
        );
        assert_eq!(ctf.to_string(), CTF, "the leading zero byte survives");
        assert_eq!(v2.to_string(), V2, "the leading zero nibble survives");
    }

    #[test]
    fn uppercase_digits_parse_and_print_lowercase() {
        let upper = CTF.to_uppercase().replacen("0X", "0x", 1);

        assert_eq!(parse(&upper).to_string(), CTF, "display is lowercase");
    }

    #[test]
    fn malformed_ids_are_rejected() {
        let bad = [
            String::new(),
            "0x".to_owned(),
            CTF_DIGITS.to_owned(),
            format!("0x{}", CTF_DIGITS.trim_end_matches('6')),
            format!("{CTF}0"),
            format!("{V2}0"),
            format!("{CTF}00"),
            format!("0xzz{}", CTF_DIGITS.trim_start_matches('0')),
            format!(" {CTF}"),
        ];
        for input in bad {
            assert!(
                input.parse::<ConditionId>().is_err(),
                "{input:?} must not parse"
            );
        }
    }

    #[test]
    fn padded_v2_form_narrows_only_when_its_last_byte_is_zero() {
        let padded = parse(&format!("{V2}00"));
        let ctf = parse(CTF);

        assert_eq!(
            padded.to_bytes31(),
            parse(V2).to_bytes31(),
            "padded form names the V2 id"
        );
        assert_ne!(padded, parse(V2), "the two widths stay distinct values");
        assert_eq!(
            ctf.to_bytes31(),
            None,
            "a nonzero last byte is not a padded V2 id"
        );
        assert_eq!(parse(V2).as_bytes32(), None, "31 bytes never widen to 32");
    }

    #[test]
    fn v2_position_id_splits_into_condition_and_outcome() {
        // A live combo trade (Data API v2, 2026-10-17): token id >> 8 is its condition id.
        let position_id = U256::from_str(
            "1778752447364788174379468102036537031873325219828321400402142166420920729600",
        )
        .expect("decimal position id");
        let no_side = position_id | U256::from(1_u8);

        assert_eq!(
            ConditionId::from_v2_position_id(position_id),
            (parse(V2), 0)
        );
        assert_eq!(ConditionId::from_v2_position_id(no_side), (parse(V2), 1));
    }

    #[test]
    fn serde_uses_the_hex_string() {
        let id: ConditionId = serde_json::from_str(&format!("\"{V2}\"")).expect("json string");

        assert_eq!(id, parse(V2));
        assert_eq!(
            serde_json::to_string(&id).expect("serialize"),
            format!("\"{V2}\"")
        );
        assert!(
            serde_json::from_str::<ConditionId>("\"0x1234\"").is_err(),
            "width checked"
        );
    }
}
