use serde::{Deserialize, Serialize};

use crate::{msg::Operation, version::ProtocolVersion};

/// IBC packet payload exchanged between the Nolus controller and the Solana
/// passive vault.
///
/// `version` pins the protocol identifier on the wire so that a mismatched
/// counterparty is rejected at deserialisation, never by business code.
///
/// Unlike the remote-lease protocol, there is no on-wire identity field: the
/// remote profit is a SINGLETON selected by port / domain / channel, so there
/// is nothing to disambiguate. The envelope therefore carries only the
/// operation, the version pin, and the nonce.
///
/// `nonce` is a per-emission identifier the Nolus profit instance chooses; the
/// controller reads it back from its own committed outbound packet on
/// ack/timeout and returns it in the callback, letting the instance reject a
/// callback that does not match the in-flight emission. `#[serde(default)]`
/// keeps the field optional at decode so an envelope authored before the
/// field existed (nonce absent) decodes to `0`; the counterparty neither
/// inspects nor echoes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PacketEnvelope {
    pub operation: Operation,
    pub version: ProtocolVersion,
    #[serde(default)]
    pub nonce: u64,
}
