use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{error::Error, response::OperationResponse};

/// Maximum byte length of a [`RemoteLeaseCallback::OperationErr`] payload.
///
/// Why a cap: the string is authored by the Solana counterparty and consumed
/// by Nolus storage / event emission. Without a bound, a hostile or
/// misbehaving counterparty can inflate event sizes and storage rows
/// arbitrarily. 512 bytes is enough to carry a structured short message
/// ("dex error: <code> <reason>") but small enough to forbid abuse.
pub const OPERATION_ERR_MAX_BYTES: usize = 512;

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
pub enum RemoteLeaseCallback {
    OperationOk(OperationResponse),
    OperationErr(RemoteErrorMessage),
    OperationTimeout,
}

/// Length-capped error string returned by the Solana counterparty.
///
/// Serialises as a bare JSON string. Deserialisation rejects payloads above
/// [`OPERATION_ERR_MAX_BYTES`] so storage writes downstream are bounded by
/// construction.
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
            Ok(Self(message))
        } else {
            Err(Error::CallbackErrorTooLong {
                actual,
                max: OPERATION_ERR_MAX_BYTES,
            })
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
