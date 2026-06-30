//! In-test stand-in for the `remote_profit_controller` contract.
//!
//! Used by the profit lifecycle integration tests that cross the
//! profit ↔ remote-profit boundary. Like its lease twin
//! (`remote_lease_controller_stub`), the stand-in is **not** a mock — it is a
//! controller-shaped CosmWasm contract running inside `cw-multi-test` that:
//!
//! - accepts the production controller's `ExecuteMsg` shape (re-exported
//!   from `remote_profit_controller::api`) so profit's outbound stubs
//!   serialise against the real wire surface,
//! - mirrors the controller's authorisation rule (every outbound
//!   `OpenProfit` / `Swap` / `TransferOut` must come from a contract whose
//!   code id equals the configured `profit_code`),
//! - **synthesises the IBC round-trip inline** in the same transaction: on
//!   every authorised outbound call it emits a `WasmMsg::Execute` back to
//!   `info.sender` carrying `profit::msg::ExecuteMsg::RemoteProfitCallback`
//!   with the per-operation response configured for the current test (the
//!   `Delayed` mode is the exception),
//! - supports per-operation `ResponseMode::{Ok, Err(reason), Delayed,
//!   FailSync}`, set via the test-only `ExecuteMsg::SetResponseMode
//!   { op, mode }` keyed by operation name (`"open_profit"`, `"swap"`,
//!   `"transfer_out"`).
//!
//! The synthesised responses are realistic-but-fixed:
//!
//! - `OpenProfit` → `OperationResponse::OpenProfit { remote_profit_id }`
//!   carrying the Solana profit authority the establishment learns once,
//! - `Swap` → `OperationResponse::Swap { amount_out }` paying the request's
//!   configured `min_out` (the literal-floor model the buy-back asserts
//!   against), unless a one-shot `SetNextSwapOutput` override is set,
//! - `TransferOut` → `OperationResponse::TransferOut(TransferOutResponse {})`.

use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use finance::coin::CoinDTO;
use platform::contract::{Code, CodeId, external};
use remote_profit::{
    callback::{RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback},
    msg::{OpenProfitParams, SwapParams, TransferOutParams},
    response::{
        OpenProfitResponse, OperationResponse, RemoteProfitId, SwapResponse, TransferOutResponse,
    },
};
use remote_profit_controller::api::{
    ChannelResponse, ConfigResponse, InstantiateMsg as ControllerInstantiateMsg,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        self, Addr, Binary, Deps, Env, MessageInfo, StdError, StdResult, Storage, WasmMsg,
        to_json_binary,
    },
    cw_storage_plus::{Item, Map},
};
use thiserror::Error;

use super::{ADMIN, CwContractWrapper, test_case::app::App};

/// The Solana profit authority the stand-in returns on `OpenProfit`. A
/// fixed base58 string mirroring Solana's singleton profit PDA (ADR-0008):
/// there is exactly one, so it is a constant, not a per-call mint.
pub const STUB_PROFIT_AUTHORITY: &str = "So1RayProfit";

/// Operation tag used both as the storage key (`ResponseModes`,
/// `PendingCallbacks`) and as the `op` argument of the test-only
/// `SetResponseMode` / `DeliverPending` variants.
pub mod op_tag {
    pub const OPEN_PROFIT: &str = "open_profit";
    pub const SWAP: &str = "swap";
    pub const TRANSFER_OUT: &str = "transfer_out";
}

/// How the stand-in answers a given outbound operation.
///
/// Default for every operation is [`ResponseMode::Ok`]. `Err` stores the
/// reason on the same wire shape Solana would use; `Delayed` persists the
/// callback for later dispatch via [`StubExecuteMsg::DeliverPending`].
/// `FailSync` makes the stand-in's execute return an `Err` outright — the
/// outbound submessage reverts, modelling a synchronous controller failure.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    #[default]
    Ok,
    Err(RemoteErrorMessage),
    Delayed,
    FailSync,
}

/// Public stand-in `ExecuteMsg`. Production variants come straight from
/// `remote_profit_controller::api::ExecuteMsg` so profit serialises against
/// the real wire surface; the test-only variants are additive.
///
/// `#[serde(deny_unknown_fields)]` is intentionally **not** applied so the
/// real controller's enum and the additive test variants coexist.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StubExecuteMsg {
    OpenChannel(),
    CloseChannel(),
    NewProfitCode {
        profit_code: Code,
    },
    OpenProfit {
        params: OpenProfitParams,
        timeout: finance::duration::Duration,
    },
    Swap {
        params: SwapParams,
        timeout: finance::duration::Duration,
        #[serde(default)]
        nonce: u64,
    },
    TransferOut {
        params: TransferOutParams,
        timeout: finance::duration::Duration,
        #[serde(default)]
        nonce: u64,
    },
    /// Test-only: configure the stand-in's reply for a given op tag.
    SetResponseMode {
        op: String,
        mode: ResponseMode,
    },
    /// Test-only: override the output the next happy-path `Swap` pays,
    /// consumed on use.
    SetNextSwapOutput {
        amount_out: CoinDTO<PaymentGroup>,
    },
    /// Test-only: dispatch the persisted [`ResponseMode::Delayed`] callback
    /// for the given op tag back to its original sender (the profit).
    DeliverPending {
        op: String,
    },
    /// Test-only: send an arbitrary callback to a profit, stamped with the
    /// profit's current in-flight nonce so a live leg credits it.
    InjectCallback {
        to: Addr,
        outcome: RemoteOperationOutcome,
    },
    /// Test-only: send a callback carrying a SPECIFIC nonce to a profit.
    InjectCallbackWithNonce {
        to: Addr,
        nonce: u64,
        outcome: RemoteOperationOutcome,
    },
}

/// Stand-in `QueryMsg`. The production variants mirror
/// `remote_profit_controller::api::QueryMsg`; the recorders are additive.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StubQueryMsg {
    Config(),
    Channel(),
    ProtocolPackageRelease {},
    /// Report every `SwapParams` the given profit has emitted, in order.
    RecordedSwaps {
        profit: Addr,
    },
    /// Report every `TransferOutParams` the given profit has emitted, in order.
    RecordedTransferOuts {
        profit: Addr,
    },
    /// Report the `OpenProfitParams` of every `OpenProfit` the given profit
    /// has emitted, in order.
    RecordedOpens {
        profit: Addr,
    },
}

const CONFIG: Item<StubConfig> = Item::new("stub_config");
const MODES: Map<&str, ResponseMode> = Map::new("stub_modes");
const PENDING: Map<&str, PendingCallback> = Map::new("stub_pending");
const RECORDED_SWAPS: Map<&Addr, Vec<SwapParams>> = Map::new("stub_recorded_swaps");
const RECORDED_TRANSFER_OUTS: Map<&Addr, Vec<TransferOutParams>> =
    Map::new("stub_recorded_transfer_outs");
const RECORDED_OPENS: Map<&Addr, Vec<OpenProfitParams>> = Map::new("stub_recorded_opens");
/// The last nonce the profit emitted on any nonce-bearing operation (swap or
/// transfer-out) — its current in-flight nonce, so an injected callback can be
/// stamped to match whatever leg is live.
const LAST_INFLIGHT_NONCE: Map<&Addr, u64> = Map::new("stub_last_inflight_nonce");
/// A one-shot output override for the next happy-path `Swap`, consumed on use.
const NEXT_SWAP_OUTPUT: Item<CoinDTO<PaymentGroup>> = Item::new("stub_next_swap_output");

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StubConfig {
    connection_id: String,
    dex_label: String,
    transfer_channel: String,
    profit_code: Code,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PendingCallback {
    sender: Addr,
    callback: RemoteProfitCallback,
}

#[derive(Error, Debug)]
pub enum StubError {
    #[error("stub config not initialised")]
    NotInitialised,
    #[error("caller {caller} is not registered or has the wrong code id")]
    Unauthorised { caller: Addr },
    #[error("no pending callback persisted for op `{op}`")]
    NoPending { op: String },
    #[error("op `{op}` is configured to fail synchronously")]
    SyncFailure { op: String },
    #[error("platform: {0}")]
    Platform(#[from] platform::error::Error),
    #[error("std: {0}")]
    Std(#[from] StdError),
}

pub fn instantiate(
    deps: cosmwasm_std::DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: ControllerInstantiateMsg,
) -> Result<CwResponse, StubError> {
    let profit_code = msg
        .profit_code
        .try_validate(&platform::contract::validator(deps.querier))?;
    let config = StubConfig {
        connection_id: msg.connection_id,
        dex_label: msg.dex_label,
        transfer_channel: msg.transfer_channel,
        profit_code,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(CwResponse::new())
}

pub fn execute(
    deps: cosmwasm_std::DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: StubExecuteMsg,
) -> Result<CwResponse, StubError> {
    match msg {
        StubExecuteMsg::OpenChannel() | StubExecuteMsg::CloseChannel() => Ok(CwResponse::new()),
        StubExecuteMsg::NewProfitCode { profit_code } => {
            CONFIG.update(deps.storage, |existing| -> Result<_, StubError> {
                Ok(StubConfig {
                    profit_code,
                    ..existing
                })
            })?;
            Ok(CwResponse::new())
        }
        StubExecuteMsg::OpenProfit { params, .. } => {
            record_open(deps.storage, &info.sender, &params)?;
            handle_outbound(deps, info, op_tag::OPEN_PROFIT, 0, |_storage| {
                Ok(OperationResponse::OpenProfit(OpenProfitResponse {
                    remote_profit_id: RemoteProfitId::new(STUB_PROFIT_AUTHORITY)
                        .map_err(|err| StubError::Std(StdError::msg(err.to_string())))?,
                }))
            })
        }
        StubExecuteMsg::Swap { params, nonce, .. } => {
            record_swap(deps.storage, &info.sender, &params)?;
            record_inflight_nonce(deps.storage, &info.sender, nonce)?;
            handle_outbound(deps, info, op_tag::SWAP, nonce, |storage| {
                Ok(OperationResponse::Swap(SwapResponse {
                    amount_out: next_swap_output(storage, &params)?,
                }))
            })
        }
        StubExecuteMsg::TransferOut { params, nonce, .. } => {
            record_transfer_out(deps.storage, &info.sender, &params)?;
            record_inflight_nonce(deps.storage, &info.sender, nonce)?;
            handle_outbound(deps, info, op_tag::TRANSFER_OUT, nonce, |_storage| {
                Ok(OperationResponse::TransferOut(TransferOutResponse {}))
            })
        }
        StubExecuteMsg::SetResponseMode { op, mode } => {
            MODES.save(deps.storage, op.as_str(), &mode)?;
            Ok(CwResponse::new())
        }
        StubExecuteMsg::SetNextSwapOutput { amount_out } => {
            NEXT_SWAP_OUTPUT.save(deps.storage, &amount_out)?;
            Ok(CwResponse::new())
        }
        StubExecuteMsg::DeliverPending { op } => deliver_pending(deps.storage, op.as_str()),
        StubExecuteMsg::InjectCallback { to, outcome } => {
            let nonce = LAST_INFLIGHT_NONCE
                .may_load(deps.storage, &to)?
                .unwrap_or(0);
            Ok(CwResponse::new()
                .add_message(callback_msg(to, RemoteProfitCallback { nonce, outcome })?))
        }
        StubExecuteMsg::InjectCallbackWithNonce { to, nonce, outcome } => Ok(CwResponse::new()
            .add_message(callback_msg(to, RemoteProfitCallback { nonce, outcome })?)),
    }
}

pub fn query(deps: Deps<'_>, _env: Env, msg: StubQueryMsg) -> StdResult<Binary> {
    match msg {
        StubQueryMsg::Config() => {
            let config = CONFIG
                .load(deps.storage)
                .map_err(|_err| StubError::NotInitialised)
                .map_err(|err| StdError::msg(err.to_string()))?;
            to_json_binary(&ConfigResponse::new(
                config.connection_id,
                config.dex_label,
                config.transfer_channel,
                config.profit_code,
                String::new(),
            ))
        }
        StubQueryMsg::Channel() => to_json_binary(&ChannelResponse { channel: None }),
        StubQueryMsg::ProtocolPackageRelease {} => Err(StdError::msg(
            "stand-in does not implement ProtocolPackageRelease",
        )),
        StubQueryMsg::RecordedSwaps { profit } => to_json_binary(
            &RECORDED_SWAPS
                .may_load(deps.storage, &profit)?
                .unwrap_or_default(),
        ),
        StubQueryMsg::RecordedTransferOuts { profit } => to_json_binary(
            &RECORDED_TRANSFER_OUTS
                .may_load(deps.storage, &profit)?
                .unwrap_or_default(),
        ),
        StubQueryMsg::RecordedOpens { profit } => to_json_binary(
            &RECORDED_OPENS
                .may_load(deps.storage, &profit)?
                .unwrap_or_default(),
        ),
    }
}

fn handle_outbound<F>(
    deps: cosmwasm_std::DepsMut<'_>,
    info: MessageInfo,
    op: &str,
    nonce: u64,
    build_ok: F,
) -> Result<CwResponse, StubError>
where
    F: FnOnce(&mut dyn Storage) -> Result<OperationResponse, StubError>,
{
    let config = CONFIG
        .load(deps.storage)
        .map_err(|_load_err| StubError::NotInitialised)?;
    require_profit_code(deps.as_ref(), &info, &config)?;

    let mode = MODES
        .may_load(deps.storage, op)?
        .unwrap_or(ResponseMode::Ok);

    let outcome = match mode {
        ResponseMode::Ok => RemoteOperationOutcome::OperationOk(build_ok(deps.storage)?.into()),
        ResponseMode::Err(reason) => RemoteOperationOutcome::OperationErr(reason),
        ResponseMode::Delayed => {
            let payload = RemoteOperationOutcome::OperationOk(build_ok(deps.storage)?.into());
            PENDING.save(
                deps.storage,
                op,
                &PendingCallback {
                    sender: info.sender.clone(),
                    callback: RemoteProfitCallback {
                        nonce,
                        outcome: payload,
                    },
                },
            )?;
            return Ok(CwResponse::new());
        }
        ResponseMode::FailSync => return Err(StubError::SyncFailure { op: op.to_owned() }),
    };

    Ok(CwResponse::new().add_message(callback_msg(
        info.sender,
        RemoteProfitCallback { nonce, outcome },
    )?))
}

/// The output the next happy-path swap pays: a one-shot [`NEXT_SWAP_OUTPUT`]
/// override if set, otherwise the request's `min_out` floor (the literal-floor
/// model the buy-back asserts against).
fn next_swap_output(
    storage: &mut dyn Storage,
    params: &SwapParams,
) -> Result<CoinDTO<PaymentGroup>, StubError> {
    match NEXT_SWAP_OUTPUT.may_load(storage)? {
        Some(amount_out) => {
            NEXT_SWAP_OUTPUT.remove(storage);
            Ok(amount_out)
        }
        None => Ok(*params.min_out()),
    }
}

fn record_swap(
    storage: &mut dyn Storage,
    sender: &Addr,
    params: &SwapParams,
) -> Result<(), StubError> {
    RECORDED_SWAPS
        .update(storage, sender, |recorded| -> Result<_, StdError> {
            let mut recorded = recorded.unwrap_or_default();
            recorded.push(params.clone());
            Ok(recorded)
        })
        .map(|_recorded| ())
        .map_err(Into::into)
}

fn record_transfer_out(
    storage: &mut dyn Storage,
    sender: &Addr,
    params: &TransferOutParams,
) -> Result<(), StubError> {
    RECORDED_TRANSFER_OUTS
        .update(storage, sender, |recorded| -> Result<_, StdError> {
            let mut recorded = recorded.unwrap_or_default();
            recorded.push(params.clone());
            Ok(recorded)
        })
        .map(|_recorded| ())
        .map_err(Into::into)
}

fn record_open(
    storage: &mut dyn Storage,
    sender: &Addr,
    params: &OpenProfitParams,
) -> Result<(), StubError> {
    RECORDED_OPENS
        .update(storage, sender, |recorded| -> Result<_, StdError> {
            let mut recorded = recorded.unwrap_or_default();
            recorded.push(params.clone());
            Ok(recorded)
        })
        .map(|_recorded| ())
        .map_err(Into::into)
}

fn record_inflight_nonce(
    storage: &mut dyn Storage,
    sender: &Addr,
    nonce: u64,
) -> Result<(), StubError> {
    LAST_INFLIGHT_NONCE
        .save(storage, sender, &nonce)
        .map_err(Into::into)
}

fn require_profit_code(
    deps: Deps<'_>,
    info: &MessageInfo,
    config: &StubConfig,
) -> Result<(), StubError> {
    use platform::contract::Validator as _;
    platform::contract::validator(deps.querier)
        .check_contract_code(info.sender.clone(), &config.profit_code)
        .map(|_ok| ())
        .map_err(|_validator_err| StubError::Unauthorised {
            caller: info.sender.clone(),
        })
}

fn deliver_pending(storage: &mut dyn Storage, op: &str) -> Result<CwResponse, StubError> {
    let pending = PENDING
        .may_load(storage, op)?
        .ok_or_else(|| StubError::NoPending { op: op.to_owned() })?;
    PENDING.remove(storage, op);
    Ok(CwResponse::new().add_message(callback_msg(pending.sender, pending.callback)?))
}

fn callback_msg(profit: Addr, callback: RemoteProfitCallback) -> StdResult<WasmMsg> {
    let payload = profit::msg::ExecuteMsg::RemoteProfitCallback(callback);
    to_json_binary(&payload).map(|encoded| WasmMsg::Execute {
        contract_addr: profit.into_string(),
        msg: encoded,
        funds: vec![],
    })
}

pub struct Instantiator;

impl Instantiator {
    /// Instantiates the stand-in. `profit_code` is the same `Code` the profit
    /// contract is registered under in the test app — used for the stand-in's
    /// authorisation check.
    #[track_caller]
    pub fn instantiate(app: &mut App, profit_code: Code) -> Addr {
        let endpoints = CwContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(endpoints));

        let msg = ControllerInstantiateMsg {
            protocol_admin: sdk::testing::user(ADMIN).into_string(),
            connection_id: super::test_case::TestCase::DEX_CONNECTION_ID.into(),
            dex_label: "test-dex".into(),
            transfer_channel: "channel-0".into(),
            profit_code: external::Code::from(CodeId::from(profit_code)),
            profit_contract: sdk::testing::user("profit-placeholder").into_string(),
        };

        app.instantiate(
            code_id,
            sdk::testing::user(ADMIN),
            &msg,
            &[],
            "remote_profit_controller_stub",
            None,
        )
        .map(|response| response.unwrap_response())
        .expect("stub controller must instantiate")
    }
}

/// Send a `SetResponseMode` to the stub.
pub fn set_response_mode(app: &mut App, controller: &Addr, op: &str, mode: ResponseMode) {
    let msg = StubExecuteMsg::SetResponseMode {
        op: op.to_owned(),
        mode,
    };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| {
            let _ = response.unwrap_response();
        })
        .expect("SetResponseMode must succeed against the stand-in");
}

/// Override the output the next happy-path swap pays, consumed on use.
pub fn set_next_swap_output(app: &mut App, controller: &Addr, amount_out: CoinDTO<PaymentGroup>) {
    let msg = StubExecuteMsg::SetNextSwapOutput { amount_out };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| {
            let _ = response.unwrap_response();
        })
        .expect("SetNextSwapOutput must succeed against the stand-in");
}

/// Trigger delivery of a previously stored Delayed callback for the given op.
pub fn deliver_pending_callback(
    app: &mut App,
    controller: &Addr,
    op: &str,
) -> sdk::cw_multi_test::AppResponse {
    let msg = StubExecuteMsg::DeliverPending { op: op.to_owned() };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| response.unwrap_response())
        .expect("DeliverPending must succeed against the stand-in")
}

/// Report every `SwapParams` the given profit has emitted.
pub fn recorded_swaps(app: &App, controller: &Addr, profit: &Addr) -> Vec<SwapParams> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedSwaps {
                profit: profit.clone(),
            },
        )
        .expect("RecordedSwaps must succeed against the stand-in")
}

/// Report every `TransferOutParams` the given profit has emitted.
pub fn recorded_transfer_outs(
    app: &App,
    controller: &Addr,
    profit: &Addr,
) -> Vec<TransferOutParams> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedTransferOuts {
                profit: profit.clone(),
            },
        )
        .expect("RecordedTransferOuts must succeed against the stand-in")
}

/// Report every `OpenProfitParams` the given profit has emitted.
pub fn recorded_opens(app: &App, controller: &Addr, profit: &Addr) -> Vec<OpenProfitParams> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedOpens {
                profit: profit.clone(),
            },
        )
        .expect("RecordedOpens must succeed against the stand-in")
}
