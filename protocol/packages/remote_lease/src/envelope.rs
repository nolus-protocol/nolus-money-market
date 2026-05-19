use serde::{Deserialize, Serialize};

pub use remote_lease_wire::envelope::LeaseAddrOnWire;
use remote_lease_wire::version::ProtocolVersion;

use crate::msg::Operation;

/// IBC packet payload exchanged between the Nolus controller and the Solana
/// passive vault.
///
/// `version` pins the protocol identifier on the wire so that a mismatched
/// counterparty is rejected at deserialisation, never by business code.
/// `lease` carries the Nolus-side lease address as an on-wire string; the
/// receiver MUST convert it through [`NolusLeaseAddr::into_validated`] before
/// using it for dispatch — the type prevents accidental use of the raw string
/// as a `cosmwasm_std::Addr`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PacketEnvelope {
    pub lease: LeaseAddrOnWire,
    pub operation: Operation,
    pub version: ProtocolVersion,
}

/// Nolus-side validation extension for [`LeaseAddrOnWire`].
///
/// `LeaseAddrOnWire` lives in the wire crate, which deliberately has no
/// CosmWasm dependency. The Nolus controller, which does have a `dyn Api`
/// available, calls [`into_validated`](NolusLeaseAddr::into_validated) to
/// produce a `cosmwasm_std::Addr`. Format validation alone is not
/// authorisation — the controller still has to verify the resulting `Addr`
/// belongs to a registered Lease instance before dispatching state-mutating
/// logic.
#[cfg(feature = "stub")]
pub trait NolusLeaseAddr {
    fn into_validated(
        self,
        api: &dyn sdk::cosmwasm_std::Api,
    ) -> sdk::cosmwasm_std::StdResult<sdk::cosmwasm_std::Addr>;
}

#[cfg(feature = "stub")]
impl NolusLeaseAddr for LeaseAddrOnWire {
    fn into_validated(
        self,
        api: &dyn sdk::cosmwasm_std::Api,
    ) -> sdk::cosmwasm_std::StdResult<sdk::cosmwasm_std::Addr> {
        api.addr_validate(self.as_str())
    }
}
