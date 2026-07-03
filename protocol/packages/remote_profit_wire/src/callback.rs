use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{error::Error, response::OperationResponse};

/// Maximum byte length of a [`RemoteOperationOutcome::OperationErr`] payload.
///
/// Why a cap: the string is authored by the Solana counterparty and consumed
/// by Nolus storage / event emission. Without a bound, a hostile or
/// misbehaving counterparty can inflate event sizes and storage rows
/// arbitrarily. 512 bytes is enough to carry a structured short message
/// ("dex error: <code> <reason>") but small enough to forbid abuse.
pub const OPERATION_ERR_MAX_BYTES: usize = 512;

/// A remote operation outcome paired with the nonce of the emission it
/// resolves.
///
/// The controller reads `nonce` back from its own committed outbound packet
/// on ack/timeout (never from the counterparty's reply) and returns it here,
/// so the profit instance can credit the outcome to the exact in-flight
/// emission and reject a duplicate, stale, or heal-superseded callback.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RemoteProfitCallback {
    pub nonce: u64,
    pub outcome: RemoteOperationOutcome,
}

/// Outcome of a remote operation as reported back to the Nolus controller.
///
/// `OperationOk` carries the typed response when Solana confirmed the
/// requested action. `OperationErr` carries a short error message authored
/// by the Solana program itself, e.g. a DEX-layer failure or an invariant
/// rejection in the vault. `OperationTimeout` is emitted by the IBC layer
/// when the packet was never acknowledged — it is structurally distinct
/// from `OperationErr` because the recovery path differs (funds may still
/// be in flight on the Solana side until the channel times out).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteOperationOutcome {
    OperationOk(OperationResponse),
    OperationErr(RemoteErrorMessage),
    OperationTimeout,
}

/// Length-capped error string returned by the Solana counterparty.
///
/// Serialises as a bare JSON string. The counterparty-facing paths —
/// deserialisation and the fallible [`new`](Self::new) — reject payloads
/// above [`OPERATION_ERR_MAX_BYTES`], so any string sourced from over the
/// wire is bounded before it reaches downstream storage. Two infallible
/// constructors complement [`new`](Self::new):
/// [`truncated`](Self::truncated) cuts an over-cap counterparty string down
/// to the cap on a UTF-8 char boundary — used where the ack path must never
/// reject and strand a committed packet — and
/// [`from_static`](Self::from_static) trusts a compile-time literal and only
/// `debug_assert!`s the bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteErrorMessage(String);

impl RemoteErrorMessage {
    pub fn new<S>(message: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let message: String = message.into();
        let actual = message.len();
        if actual <= OPERATION_ERR_MAX_BYTES {
            let value = Self(message);
            debug_assert!(value.invariant_held());
            Ok(value)
        } else {
            Err(Error::CallbackErrorTooLong {
                actual,
                max: OPERATION_ERR_MAX_BYTES,
            })
        }
    }

    /// Construct from a counterparty-authored string, cutting it down to at
    /// most [`OPERATION_ERR_MAX_BYTES`] on a UTF-8 char boundary.
    ///
    /// Unlike [`new`](Self::new), this is total: an over-cap message is
    /// truncated rather than rejected. The ack path uses it so a counterparty
    /// error string can never fail construction and strand an already-committed
    /// packet behind an endless relayer retry.
    pub fn truncated<S>(message: S) -> Self
    where
        S: Into<String>,
    {
        let mut message: String = message.into();
        let end = message
            .char_indices()
            .map(|(start, ch)| start + ch.len_utf8())
            .take_while(|boundary| *boundary <= OPERATION_ERR_MAX_BYTES)
            .last()
            .unwrap_or(0);
        message.truncate(end);
        let value = Self(message);
        debug_assert!(value.invariant_held());
        value
    }

    /// Construct from a compile-time-known string literal that is statically
    /// known to be within [`OPERATION_ERR_MAX_BYTES`].
    ///
    /// For fixed internal reasons (e.g. `"timeout"`) where threading a
    /// fallible [`new`](Self::new) through the call site would add error
    /// plumbing for a value that is provably in range.
    ///
    /// Precondition (caller's responsibility): `message` must be a genuine
    /// literal whose length is verifiable by inspection, never a
    /// runtime-produced `&'static str` (e.g. a `Box::leak`ed value). The
    /// length is only `debug_assert!`ed, so an over-cap input that slips past
    /// review would bypass the bound in release builds — unlike [`new`] and
    /// deserialisation, which reject it. When the length is not statically
    /// obvious, use [`new`] instead.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `message` exceeds [`OPERATION_ERR_MAX_BYTES`].
    pub fn from_static(message: &'static str) -> Self {
        let value = Self(message.to_owned());
        debug_assert!(
            value.invariant_held(),
            "RemoteErrorMessage::from_static exceeds OPERATION_ERR_MAX_BYTES"
        );
        value
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn invariant_held(&self) -> bool {
        self.0.len() <= OPERATION_ERR_MAX_BYTES
    }
}

impl Serialize for RemoteErrorMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RemoteErrorMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Rejects payloads above the cap inside the visitor, before any
        // owned `String` is materialised on our side. With `serde_json`'s
        // `deserialize_str` the visitor receives a borrowed slice into the
        // input buffer when no JSON escapes are present, so the over-cap
        // case allocates nothing beyond the already-bounded IBC packet.
        deserializer.deserialize_str(RemoteErrorMessageVisitor)
    }
}

struct RemoteErrorMessageVisitor;

impl RemoteErrorMessageVisitor {
    fn take_within_cap<E, F>(&self, len: usize, take: F) -> Result<RemoteErrorMessage, E>
    where
        E: de::Error,
        F: FnOnce() -> String,
    {
        (len <= OPERATION_ERR_MAX_BYTES)
            .then(take)
            .map(RemoteErrorMessage)
            .ok_or_else(|| E::invalid_length(len, self))
    }
}

impl<'de> de::Visitor<'de> for RemoteErrorMessageVisitor {
    type Value = RemoteErrorMessage;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a string of at most {OPERATION_ERR_MAX_BYTES} bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.take_within_cap(value.len(), || value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let len = value.len();
        self.take_within_cap(len, || value)
    }
}
