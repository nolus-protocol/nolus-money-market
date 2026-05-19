use serde::{Deserialize, Serialize};

use crate::ticker::Ticker;

/// Coin amount as it travels on the wire.
///
/// Type-level alias mirroring `finance::coin::Amount` (`u128`). Defined locally
/// so the wire crate stays free of the `finance` dependency tree — the wire
/// format is the contract here, not Rust-level identity.
pub type Amount = u128;

/// Wire encoding of a coin: a `u128` amount + a ticker.
///
/// JSON shape matches `finance::CoinDTO<G>`:
/// `{"amount":"<u128-decimal>","ticker":"<TICKER>"}`. The amount is encoded as
/// a quoted decimal string (no leading zeros, no sign) so values above `2^53`
/// survive JSON parsers that materialise numbers as `f64`. Deserialisation
/// rejects non-canonical encodings (empty, leading-zero, non-digit).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WireCoin {
    #[serde(with = "string_amount")]
    amount: Amount,
    ticker: Ticker,
}

impl WireCoin {
    pub const fn new(amount: Amount, ticker: Ticker) -> Self {
        Self { amount, ticker }
    }

    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn ticker(&self) -> &Ticker {
        &self.ticker
    }

    pub const fn is_zero(&self) -> bool {
        self.amount == 0
    }
}

mod string_amount {
    //! Quoted-decimal `u128` serde, byte-compatible with `finance::CoinDTO`.
    //!
    //! Rejects non-canonical inputs at deserialise time: empty, leading-zero
    //! (except the single-character `"0"`), or non-digit. Two byte sequences
    //! must never decode to the same value — important for any downstream
    //! consumer that hashes the JSON for canonical signing / replay defence.

    use std::fmt;

    use serde::{Deserializer, Serializer, de};

    use super::Amount;

    pub(super) fn serialize<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&amount.to_string())
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AmountVisitor)
    }

    struct AmountVisitor;

    impl de::Visitor<'_> for AmountVisitor {
        type Value = Amount;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a canonical decimal u128 in JSON string form")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            canonical(v)
                .then(|| v.parse::<Amount>().ok())
                .flatten()
                .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    fn canonical(v: &str) -> bool {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        v == "0" || !v.starts_with('0')
    }
}
