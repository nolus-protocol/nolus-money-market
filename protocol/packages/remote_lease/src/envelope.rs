use serde::{Deserialize, Serialize};

use crate::{msg::LeaseOperationsMsg, version::ProtocolVersion};

/// IBC packet payload exchanged between the Nolus controller and the Solana
/// passive vault.
///
/// `version` pins the protocol identifier on the wire so that a mismatched
/// counterparty is rejected at deserialisation, never by business code.
/// `lease` carries the Nolus-side lease address as an on-wire string; the
/// receiver MUST convert it through [`LeaseAddrOnWire::into_validated`] before
/// using it for dispatch — the type prevents accidental use of the raw string
/// as a `cosmwasm_std::Addr`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PacketEnvelope {
    pub lease: LeaseAddrOnWire,
    pub operation: LeaseOperationsMsg,
    pub version: ProtocolVersion,
}

/// On-wire encoding of a Nolus lease address.
///
/// Serialises as a bare JSON string (`#[serde(transparent)]`). The inner
/// string is intentionally inaccessible: the only ways to obtain a usable
/// address from a received envelope are
/// [`LeaseAddrOnWire::into_validated`] (CosmWasm consumers, gated by the
/// `stub` feature) and [`LeaseAddrOnWire::as_str`] (display / logging only).
/// Format validation alone is not authorisation — the controller still has to
/// verify the resulting `Addr` belongs to a registered Lease instance before
/// dispatching state-mutating logic.
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

    #[cfg(feature = "stub")]
    pub fn into_validated(
        self,
        api: &dyn sdk::cosmwasm_std::Api,
    ) -> sdk::cosmwasm_std::StdResult<sdk::cosmwasm_std::Addr> {
        api.addr_validate(&self.0)
    }
}
