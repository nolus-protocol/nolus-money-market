use serde::{Deserialize, Serialize};

pub use remote_profit_wire::callback::{OPERATION_ERR_MAX_BYTES, RemoteErrorMessage};

use crate::response::WireOperationResponse;

/// A remote operation outcome paired with the nonce of the emission it
/// resolves.
///
/// The controller reads `nonce` back from its own committed outbound packet
/// on ack/timeout (never from the counterparty's reply) and returns it here,
/// so the addressee profit instance can credit the outcome to the exact
/// in-flight emission and reject a duplicate, stale, or heal-superseded
/// callback.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RemoteProfitCallback {
    pub nonce: u64,
    pub outcome: RemoteOperationOutcome,
}

/// Outcome of a remote operation as reported back to the Nolus controller.
///
/// `OperationOk` carries the wire-shaped response verbatim when Solana
/// confirmed the requested action: the controller validates only that the
/// payload is a well-formed response, while content validation (the currency
/// registry) belongs to the addressee profit instance, whose callback handlers
/// absorb failures instead of erring (ADR 0001 §3.7.2). `OperationErr` carries
/// a short error message authored by the Solana program itself, e.g. a
/// DEX-layer failure or an invariant rejection in the vault.
/// `OperationTimeout` is emitted by the IBC layer when the packet was never
/// acknowledged — it is structurally distinct from `OperationErr` because
/// the recovery path differs (funds may still be in flight on the Solana
/// side until the channel times out).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteOperationOutcome {
    OperationOk(WireOperationResponse),
    OperationErr(RemoteErrorMessage),
    OperationTimeout,
}
