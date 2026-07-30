use currency::Group;
use serde::{Deserialize, Serialize};

pub use remote_lease_wire::callback::{
    OPERATION_ERR_MAX_BYTES, REMOTE_ERROR_CODE_FRAME_MAX_BYTES, REMOTE_ERROR_CODE_MAX_BYTES,
    RemoteError, RemoteErrorKind, RemoteErrorMessage,
};

use crate::response::OperationResponse;

/// Outcome of a remote operation as reported back to the Nolus controller.
///
/// `OperationOk` carries the typed response when Solana confirmed the
/// requested action. `OperationErr` carries the counterparty's classified
/// failure — a [`RemoteErrorKind`] to branch on plus its own prose.
/// `OperationTimeout` is emitted by the IBC layer when the packet was never
/// acknowledged — it is structurally distinct from `OperationErr` because the
/// recovery path differs (funds may still be in flight on the Solana side
/// until the channel times out).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RemoteLeaseCallback<G>
where
    G: Group,
{
    OperationOk(OperationResponse<G>),
    OperationErr(RemoteError),
    OperationTimeout,
}
