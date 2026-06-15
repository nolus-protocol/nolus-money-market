use serde::{Deserialize, Serialize};

use crate::{msg::Operation, version::ProtocolVersion};

/// IBC packet payload exchanged between the Nolus controller and the Solana
/// passive vault.
///
/// `version` pins the protocol identifier on the wire so that a mismatched
/// counterparty is rejected at deserialisation, never by business code.
/// `lease` carries the Nolus-side lease address as an on-wire string; Nolus
/// consumers MUST validate the string through their own `Addr` constructor
/// before using it for dispatch — the type prevents accidental use of the raw
/// string as an authenticated address.
///
/// `nonce` is a per-emission identifier the Nolus lease chooses; the
/// controller reads it back from its own committed outbound packet on
/// ack/timeout and returns it in the callback, letting the lease reject a
/// callback that does not match the in-flight emission. `#[serde(default)]`
/// keeps the field optional at decode so an envelope authored before the
/// field existed (nonce absent) decodes to `0`; the counterparty neither
/// inspects nor echoes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PacketEnvelope {
    pub lease: LeaseAddrOnWire,
    pub operation: Operation,
    pub version: ProtocolVersion,
    #[serde(default)]
    pub nonce: u64,
}

/// On-wire encoding of a Nolus lease address.
///
/// Serialises as a bare JSON string (`#[serde(transparent)]`). The inner
/// string is intentionally inaccessible: the only way to inspect the address
/// without converting through Nolus address validation is [`as_str`], which
/// is intended for display / logging only. Format validation alone is not
/// authorisation — the controller still has to verify the resulting address
/// belongs to a registered Lease instance before dispatching state-mutating
/// logic.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(transparent)]
pub struct LeaseAddrOnWire(String);

impl LeaseAddrOnWire {
    pub fn new<S>(addr: S) -> Self
    where
        S: Into<String>,
    {
        Self(addr.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
