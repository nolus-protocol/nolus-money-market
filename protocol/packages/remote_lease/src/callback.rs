use serde::{Deserialize, Serialize};

pub use remote_lease_wire::callback::{OPERATION_ERR_MAX_BYTES, RemoteErrorMessage};

use crate::response::OperationResponse;

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
