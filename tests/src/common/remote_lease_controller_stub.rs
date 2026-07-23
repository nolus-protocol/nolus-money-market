//! In-test stand-in for the `remote_lease_controller` contract.
//!
//! Used by every Lease lifecycle integration test that crosses the
//! Lease ↔ remote-lease boundary. The stand-in is **not** a mock — it is a
//! controller-shaped CosmWasm contract running inside `cw-multi-test` that:
//!
//! - accepts the production controller's `ExecuteMsg` shape (re-exported
//!   from `remote_lease_controller::api`) so the lease's outbound stubs
//!   serialise against the real wire surface,
//! - mirrors the controller's authorisation rule (every outbound
//!   `OpenLease` / `CloseLease` / `Swap` / `TransferOut` must come from a
//!   contract whose code id equals the configured `lease_code`),
//! - **synthesises the IBC round-trip inline** in the same transaction:
//!   on every authorised outbound call it emits a `WasmMsg::Execute` back
//!   to `info.sender` carrying `lease::api::ExecuteMsg::RemoteLeaseCallback`
//!   with the per-operation response configured for the current test (the
//!   `Delayed` mode is the exception — see below),
//! - supports per-operation `ResponseMode::{Ok, Err(reason), Delayed}`,
//!   set via the test-only `ExecuteMsg::SetResponseMode { op, mode }` and
//!   stored in `ResponseModes` keyed by operation name (`"open_lease"`,
//!   `"close_lease"`, `"swap"`, `"transfer_out"`),
//! - in `Delayed` mode persists the would-be callback (operation name,
//!   sender, payload) into `PendingCallbacks` so the test can advance
//!   blocks and then dispatch via `ExecuteMsg::DeliverPending { op }`.
//!
//! The synthesised responses are realistic-but-fixed:
//!
//! - `OpenLease` → `OperationResponse::OpenLease { remote_lease_id }`
//!   with a synthetic but valid PDA-looking string (the stub mints a fresh
//!   one per `OpenLease` call to mirror Solana's unique-per-lease PDA),
//! - `Swap` → `OperationResponse::Swap { amount_out }` where `amount_out`
//!   is governed by the test-settable [`SwapFill`] (default [`SwapFill::MinOut`]
//!   pays the request's `min_out`; [`SwapFill::InputAmount`] pays the summed
//!   input in the `min_out` currency — an identity DEX fill; [`SwapFill::Fixed`]
//!   pays a caller-chosen absolute amount). Every `Swap` request is captured
//!   and exposed via the test-only [`StubQueryMsg::CapturedSwap`],
//! - `TransferOut` → `OperationResponse::TransferOut(TransferOutResponse {})`,
//!   `CloseLease` → `OperationResponse::CloseLease(CloseLeaseResponse {})`.
//!
//! Phase 3-6 of issue #142 wire the lease state machine to actually call
//! these stub methods. The stand-in itself compiles and exercises today
//! against the unchanged callback surface (issue #141 / PR #631).

use serde::{Deserialize, Serialize};

use currencies::{LeaseGroup, Lpns, PaymentGroup};
use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin};
use platform::contract::{Code, CodeId};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    msg::{CloseLeaseParams, OpenLeaseParams, TransferOutParams},
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId, SwapResponse,
        TransferOutResponse,
    },
    swap::SwapParams,
};
use remote_lease_controller::api::{
    ChannelResponse, ConfigResponse, InstantiateMsg as ControllerInstantiateMsg,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        self, Addr, Binary, Deps, Env, MessageInfo, StdError, StdResult, Storage, Uint64, WasmMsg,
        to_json_binary,
    },
    cw_multi_test::AppResponse,
    cw_storage_plus::{Item, Map},
};
use thiserror::Error;

use super::{
    ADMIN, CwContractWrapper,
    test_case::{app::App, response::ResponseWithInterChainMsgs},
};

/// Operation tag used both as the storage key (`ResponseModes`,
/// `PendingCallbacks`) and as the `op` argument of the test-only
/// `SetResponseMode` / `DeliverPending` variants. Mirroring snake_case
/// matches the controller's wire idiom.
pub mod op_tag {
    pub const OPEN_LEASE: &str = "open_lease";
    pub const CLOSE_LEASE: &str = "close_lease";
    pub const SWAP: &str = "swap";
    pub const TRANSFER_OUT: &str = "transfer_out";
}

/// How the stand-in answers a given outbound operation.
///
/// Default for every operation is [`ResponseMode::Ok`]. `Err` stores the
/// reason on the same wire shape Solana would use; `Delayed` persists the
/// callback for later dispatch via [`StubExecuteMsg::DeliverPending`].
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    #[default]
    Ok,
    Err(RemoteErrorMessage),
    Delayed,
}

/// The `amount_out` a happy-path `Swap` ack pays back, in the request's
/// `min_out` currency.
///
/// The remote (Solana) DEX fill is opaque to the lease — it learns the output
/// only from the ack — so the stand-in must be told what to pay. `MinOut` (the
/// default) keeps the literal-floor model some tests rely on; `InputAmount`
/// reproduces the legacy identity fill (`|price, _, _| price`) the open flow
/// and repay use; `Fixed` names an exact outcome for close/liquidation tests
/// that pinned a specific `price_f` result.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SwapFill {
    #[default]
    MinOut,
    InputAmount,
    Fixed(Amount),
}

/// Public stand-in `ExecuteMsg` — production variants come straight from
/// `remote_lease_controller::api::ExecuteMsg` so the lease serialises against
/// the real wire surface; the test-only `SetResponseMode` and
/// `DeliverPending` variants are additive (untagged via `#[serde(untagged)]`
/// at the top level by re-encoding the controller enum as a flat variant).
///
/// `#[serde(deny_unknown_fields)]` is intentionally **not** applied so the
/// real controller's enum and the additive test variants coexist.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StubExecuteMsg {
    OpenChannel(),
    CloseChannel(),
    NewLeaseCode {
        lease_code: Code,
    },
    OpenLease {
        params: OpenLeaseParams<LeaseGroup, Lpns, PaymentGroup>,
        timeout: finance::duration::Duration,
    },
    CloseLease {
        params: CloseLeaseParams,
        timeout: finance::duration::Duration,
    },
    Swap {
        params: SwapParams<PaymentGroup, PaymentGroup>,
        timeout: finance::duration::Duration,
    },
    TransferOut {
        params: TransferOutParams<PaymentGroup>,
        timeout: finance::duration::Duration,
    },
    /// Test-only: configure the stand-in's reply for a given op tag.
    SetResponseMode {
        op: String,
        mode: ResponseMode,
    },
    /// Test-only: dispatch the persisted [`ResponseMode::Delayed`] callback
    /// for the given op tag back to its original sender (the lease).
    DeliverPending {
        op: String,
    },
    /// Test-only: set the `amount_out` a happy-path `Swap` ack pays back.
    SetSwapFill {
        fill: SwapFill,
    },
}

/// Public stand-in `QueryMsg` — a superset of
/// `remote_lease_controller::api::QueryMsg` (mirrored variant-for-variant so
/// production queries still resolve) plus the test-only `CapturedSwap`. As with
/// [`StubExecuteMsg`], `#[serde(deny_unknown_fields)]` is intentionally omitted
/// so the additive variant coexists with the real controller enum.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StubQueryMsg {
    Config(),
    Channel(),
    ProtocolPackageRelease {},
    /// Test-only: return the `SwapParams` of the most recent `Swap` request.
    CapturedSwap {},
    /// Test-only: return the number of `Swap` requests received so far.
    SwapCount {},
}

/// Stand-in state.
///
/// `config` mirrors the production controller's `Config`, minus channel
/// state and protocol-admin enforcement (the test-only `New_LeaseCode`
/// path is unauthenticated — tests are trusted). `modes` is keyed by op
/// tag and falls back to `ResponseMode::Ok` on absence. `pending` stores
/// the most recent `Delayed` callback per op tag.
const CONFIG: Item<StubConfig> = Item::new("stub_config");
const MODES: Map<&str, ResponseMode> = Map::new("stub_modes");
const PENDING: Map<&str, PendingCallback> = Map::new("stub_pending");
const LEASE_PDA_COUNTER: Item<u64> = Item::new("stub_pda_counter");
const SWAP_FILL: Item<SwapFill> = Item::new("stub_swap_fill");
const SWAP_CAPTURED: Item<SwapParams<PaymentGroup, PaymentGroup>> = Item::new("stub_swap_captured");
const SWAP_COUNT: Item<u64> = Item::new("stub_swap_count");

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StubConfig {
    connection_id: String,
    dex_label: String,
    lease_code: Code,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PendingCallback {
    sender: Addr,
    callback: RemoteLeaseCallback<PaymentGroup>,
}

#[derive(Error, Debug)]
pub enum StubError {
    #[error("stub config not initialised")]
    NotInitialised,
    #[error("caller {caller} is not registered or has the wrong code id")]
    Unauthorised { caller: Addr },
    #[error("no pending callback persisted for op `{op}`")]
    NoPending { op: String },
    #[error("std: {0}")]
    Std(#[from] StdError),
}

pub fn instantiate(
    deps: cosmwasm_std::DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: ControllerInstantiateMsg,
) -> Result<CwResponse, StubError> {
    let lease_code = Code::unchecked(u64::from(msg.lease_code));
    let config = StubConfig {
        connection_id: msg.connection_id,
        dex_label: msg.dex_label,
        lease_code,
    };
    CONFIG.save(deps.storage, &config)?;
    LEASE_PDA_COUNTER.save(deps.storage, &0)?;
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
        StubExecuteMsg::NewLeaseCode { lease_code } => {
            CONFIG.update(deps.storage, |existing| -> Result<_, StubError> {
                Ok(StubConfig {
                    lease_code,
                    ..existing
                })
            })?;
            Ok(CwResponse::new())
        }
        StubExecuteMsg::OpenLease { params, .. } => {
            handle_outbound(deps, info, op_tag::OPEN_LEASE, |storage| {
                synth_open_lease_response(storage, &params)
            })
        }
        StubExecuteMsg::CloseLease { .. } => {
            handle_outbound(deps, info, op_tag::CLOSE_LEASE, |_storage| {
                Ok(OperationResponse::CloseLease(CloseLeaseResponse {}))
            })
        }
        StubExecuteMsg::Swap { params, .. } => {
            let fill = SWAP_FILL.may_load(deps.storage)?.unwrap_or_default();
            let amount_out = swap_amount_out(&params, &fill);
            SWAP_CAPTURED.save(deps.storage, &params)?;
            let count = SWAP_COUNT.may_load(deps.storage)?.unwrap_or(0) + 1;
            SWAP_COUNT.save(deps.storage, &count)?;
            handle_outbound(deps, info, op_tag::SWAP, move |_storage| {
                Ok(OperationResponse::Swap(SwapResponse { amount_out }))
            })
        }
        StubExecuteMsg::TransferOut { .. } => {
            handle_outbound(deps, info, op_tag::TRANSFER_OUT, |_storage| {
                Ok(OperationResponse::TransferOut(TransferOutResponse {}))
            })
        }
        StubExecuteMsg::SetResponseMode { op, mode } => {
            MODES.save(deps.storage, op.as_str(), &mode)?;
            Ok(CwResponse::new())
        }
        StubExecuteMsg::DeliverPending { op } => deliver_pending(deps.storage, op.as_str()),
        StubExecuteMsg::SetSwapFill { fill } => {
            SWAP_FILL.save(deps.storage, &fill)?;
            Ok(CwResponse::new())
        }
    }
}

pub fn query(deps: Deps<'_>, _env: Env, msg: StubQueryMsg) -> StdResult<Binary> {
    match msg {
        StubQueryMsg::Config() => {
            let config = CONFIG
                .load(deps.storage)
                .map_err(|_err| StubError::NotInitialised)?;
            to_json_binary(&ConfigResponse {
                connection_id: config.connection_id,
                dex_label: config.dex_label,
                lease_code_id: CodeId::from(config.lease_code).into(),
            })
        }
        StubQueryMsg::Channel() => {
            // Channel state is not exercised by the lease-side tests —
            // the stand-in synthesises the round-trip in-process.
            to_json_binary(&ChannelResponse { channel: None })
        }
        StubQueryMsg::ProtocolPackageRelease {} => Err(StdError::msg(
            "stand-in does not implement ProtocolPackageRelease",
        )),
        StubQueryMsg::CapturedSwap {} => {
            let captured = SWAP_CAPTURED
                .may_load(deps.storage)?
                .ok_or_else(|| StdError::msg("no swap captured by the stand-in yet"))?;
            to_json_binary(&captured)
        }
        StubQueryMsg::SwapCount {} => {
            to_json_binary(&SWAP_COUNT.may_load(deps.storage)?.unwrap_or(0))
        }
    }
}

fn handle_outbound<F>(
    deps: cosmwasm_std::DepsMut<'_>,
    info: MessageInfo,
    op: &str,
    build_ok: F,
) -> Result<CwResponse, StubError>
where
    F: FnOnce(&mut dyn Storage) -> Result<OperationResponse<PaymentGroup>, StubError>,
{
    let config = CONFIG
        .load(deps.storage)
        .map_err(|_load_err| StubError::NotInitialised)?;
    require_lease_code(deps.as_ref(), &info, &config)?;

    let mode = MODES
        .may_load(deps.storage, op)?
        .unwrap_or(ResponseMode::Ok);

    let callback = match mode {
        ResponseMode::Ok => RemoteLeaseCallback::OperationOk(build_ok(deps.storage)?),
        ResponseMode::Err(reason) => RemoteLeaseCallback::OperationErr(reason),
        ResponseMode::Delayed => {
            let payload = RemoteLeaseCallback::OperationOk(build_ok(deps.storage)?);
            PENDING.save(
                deps.storage,
                op,
                &PendingCallback {
                    sender: info.sender.clone(),
                    callback: payload,
                },
            )?;
            return Ok(CwResponse::new());
        }
    };

    Ok(CwResponse::new().add_message(callback_msg(info.sender, callback)?))
}

fn require_lease_code(
    deps: Deps<'_>,
    info: &MessageInfo,
    config: &StubConfig,
) -> Result<(), StubError> {
    use platform::contract::Validator as _;
    platform::contract::validator(deps.querier)
        .check_contract_code(info.sender.clone(), &config.lease_code)
        .map(|_ok| ())
        .map_err(|_validator_err| StubError::Unauthorised {
            caller: info.sender.clone(),
        })
}

fn synth_open_lease_response(
    storage: &mut dyn Storage,
    _params: &OpenLeaseParams<LeaseGroup, Lpns, PaymentGroup>,
) -> Result<OperationResponse<PaymentGroup>, StubError> {
    let next = LEASE_PDA_COUNTER.may_load(storage)?.unwrap_or(0) + 1;
    LEASE_PDA_COUNTER.save(storage, &next)?;
    // PDA-shaped base58 placeholder; the validation in `RemoteLeaseId::new`
    // restricts characters but does not enforce on-chain shape — fine for
    // an integration stand-in. The prefix is fixed so test assertions can
    // pattern-match on it.
    // Pad with `1` (smallest valid base58 digit) — `0` is excluded from the
    // alphabet, which `RemoteLeaseId::new` rejects.
    let raw = format!("StubPda{next:1>32}");
    let id =
        RemoteLeaseId::new(raw).map_err(|err| StubError::Std(StdError::msg(err.to_string())))?;
    Ok(OperationResponse::OpenLease(OpenLeaseResponse {
        remote_lease_id: id,
    }))
}

fn swap_amount_out(
    params: &SwapParams<PaymentGroup, PaymentGroup>,
    fill: &SwapFill,
) -> CoinDTO<PaymentGroup> {
    match fill {
        SwapFill::MinOut => *params.min_out(),
        SwapFill::InputAmount => coin_out(input_sum(params), params.min_out()),
        SwapFill::Fixed(amount) => coin_out(*amount, params.min_out()),
    }
}

fn input_sum(params: &SwapParams<PaymentGroup, PaymentGroup>) -> Amount {
    match params {
        SwapParams::One { coin_in, .. } => coin_in.amount(),
        SwapParams::Two {
            coin_in_1,
            coin_in_2,
            ..
        } => coin_in_1
            .amount()
            .checked_add(coin_in_2.amount())
            .expect("swap input amounts must not overflow"),
    }
}

/// Build a `CoinDTO` carrying `amount` in the currency of `witness`. Dispatches
/// on `witness`'s runtime currency so the coin is typed with the matching
/// currency — never a wrong-currency witness.
fn coin_out(amount: Amount, witness: &CoinDTO<PaymentGroup>) -> CoinDTO<PaymentGroup> {
    witness.with_coin(WithAmount { amount })
}

struct WithAmount {
    amount: Amount,
}

impl WithCoin<PaymentGroup> for WithAmount {
    type Outcome = CoinDTO<PaymentGroup>;

    fn on<C>(self, _coin: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<PaymentGroup> + MemberOf<<PaymentGroup as Group>::TopG>,
    {
        Coin::<C>::new(self.amount).into()
    }
}

fn deliver_pending(storage: &mut dyn Storage, op: &str) -> Result<CwResponse, StubError> {
    let pending = PENDING
        .may_load(storage, op)?
        .ok_or_else(|| StubError::NoPending { op: op.to_owned() })?;
    PENDING.remove(storage, op);
    Ok(CwResponse::new().add_message(callback_msg(pending.sender, pending.callback)?))
}

fn callback_msg(lease: Addr, callback: RemoteLeaseCallback<PaymentGroup>) -> StdResult<WasmMsg> {
    let payload = lease::api::ExecuteMsg::RemoteLeaseCallback(callback);
    to_json_binary(&payload).map(|encoded| WasmMsg::Execute {
        contract_addr: lease.into_string(),
        msg: encoded,
        funds: vec![],
    })
}

pub struct Instantiator;

impl Instantiator {
    /// Instantiates the stand-in. `lease_code` is the same `Code` the
    /// lease contract is registered under in the test app — used for the
    /// stand-in's authorisation check.
    #[track_caller]
    pub fn instantiate(app: &mut App, lease_code: Code) -> Addr {
        let endpoints = CwContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(endpoints));

        let msg = ControllerInstantiateMsg {
            protocol_admin: sdk::testing::user(ADMIN).into_string(),
            connection_id: super::test_case::TestCase::DEX_CONNECTION_ID.into(),
            dex_label: "test-dex".into(),
            lease_code: Uint64::from(CodeId::from(lease_code)),
        };

        app.instantiate(
            code_id,
            sdk::testing::user(ADMIN),
            &msg,
            &[],
            "remote_lease_controller_stub",
            None,
        )
        .map(|response| response.unwrap_response())
        .expect("stub controller must instantiate")
    }
}

/// Helper for tests: send a `SetResponseMode` to the stub.
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

/// Helper for tests: trigger delivery of a previously stored Delayed
/// callback for the given op tag, returning the lease's response so the caller
/// can drain any follow-up messages (e.g. a local-output swap's transfer-in).
#[track_caller]
pub fn deliver_pending_callback<'r>(
    app: &'r mut App,
    controller: &Addr,
    op: &str,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    let msg = StubExecuteMsg::DeliverPending { op: op.to_owned() };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .expect("DeliverPending must succeed against the stand-in")
}

/// Helper for tests: set the `amount_out` a happy-path `Swap` ack pays back.
pub fn set_swap_fill(app: &mut App, controller: &Addr, fill: SwapFill) {
    let msg = StubExecuteMsg::SetSwapFill { fill };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| {
            let _ = response.unwrap_response();
        })
        .expect("SetSwapFill must succeed against the stand-in");
}

/// Helper for tests: read the `SwapParams` of the most recent `Swap` request
/// the stand-in received.
#[track_caller]
pub fn captured_swap(app: &App, controller: &Addr) -> SwapParams<PaymentGroup, PaymentGroup> {
    app.query()
        .query_wasm_smart(controller.clone(), &StubQueryMsg::CapturedSwap {})
        .expect("CapturedSwap query must succeed against the stand-in")
}

/// Helper for tests: the number of `Swap` requests the stand-in has received.
#[track_caller]
pub fn swap_count(app: &App, controller: &Addr) -> u64 {
    app.query()
        .query_wasm_smart(controller.clone(), &StubQueryMsg::SwapCount {})
        .expect("SwapCount query must succeed against the stand-in")
}
