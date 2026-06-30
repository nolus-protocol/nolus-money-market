use serde::Serialize;

use remote_profit::callback::RemoteProfitCallback;

/// On-wire shape of the profit contract's `ExecuteMsg` variant that consumes
/// the IBC callback. The profit contract is a separate crate; this private
/// shim pins the JSON the controller emits so the profit-side variant can
/// land in a follow-up PR without coupling deploy schedules. The profit's
/// own `ExecuteMsg` enum already uses `rename_all = "snake_case"`, so the
/// eventual `RemoteProfitCallback(RemoteProfitCallback)` variant will accept
/// this payload byte-for-byte.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfitExecuteMsg {
    RemoteProfitCallback(RemoteProfitCallback),
}
