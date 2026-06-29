use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::error::Error;

/// Maximum byte length of a [`RemoteProfitId`] payload.
///
/// Base58-encoded Solana addresses are 32-44 ASCII characters in practice; the
/// 64-byte cap leaves headroom while bounding event and storage size from a
/// possibly-misbehaving counterparty.
pub const REMOTE_PROFIT_ID_MAX_BYTES: usize = 64;

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Typed wrapper around the Solana profit authority address travelling on the
/// wire.
///
/// Serialises as a bare JSON string (`#[serde(transparent)]`-equivalent through
/// the manual impls below) so existing off-chain consumers that read the field
/// as a string see no shape change. Validation lives inside the constructor:
/// non-empty, at most [`REMOTE_PROFIT_ID_MAX_BYTES`] bytes, base58 character
/// set. Deserialisation enforces the same invariants — a packet with an
/// invalid id is rejected at parse time, never observed by business code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RemoteProfitId(String);

impl RemoteProfitId {
    pub fn new<S>(value: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let value: String = value.into();
        validate(&value).map(|()| Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RemoteProfitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for RemoteProfitId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

fn validate(value: &str) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::RemoteProfitIdEmpty);
    }
    let len = value.len();
    if REMOTE_PROFIT_ID_MAX_BYTES < len {
        return Err(Error::RemoteProfitIdTooLong {
            actual: len,
            max: REMOTE_PROFIT_ID_MAX_BYTES,
        });
    }
    value
        .as_bytes()
        .iter()
        .find(|byte| !BASE58_ALPHABET.contains(byte))
        .map_or(Ok(()), |byte| {
            Err(Error::RemoteProfitIdInvalidCharacter { byte: *byte })
        })
}

impl Serialize for RemoteProfitId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RemoteProfitId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RemoteProfitIdVisitor)
    }
}

struct RemoteProfitIdVisitor;

impl de::Visitor<'_> for RemoteProfitIdVisitor {
    type Value = RemoteProfitId;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a non-empty base58 string of at most {REMOTE_PROFIT_ID_MAX_BYTES} bytes"
        )
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        RemoteProfitId::new(value).map_err(|err| E::custom(err.to_string()))
    }
}
