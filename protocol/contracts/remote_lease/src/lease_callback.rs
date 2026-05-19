use serde::Serialize;

use remote_lease::callback::RemoteLeaseCallback;

/// On-wire shape of the lease contract's `ExecuteMsg` variant that consumes
/// the IBC callback. The lease contract is a separate crate; this private
/// shim pins the JSON the controller emits so the lease-side variant can
/// land in a follow-up PR without coupling deploy schedules. The lease's
/// own `ExecuteMsg` enum already uses `rename_all = "snake_case"`, so the
/// eventual `RemoteLeaseCallback(RemoteLeaseCallback)` variant will accept
/// this payload byte-for-byte.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeaseExecuteMsg {
    RemoteLeaseCallback(RemoteLeaseCallback),
}
