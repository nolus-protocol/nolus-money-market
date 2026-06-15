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
//! - supports per-operation `ResponseMode::{Ok, Err(reason), Delayed,
//!   FailSync}`, set via the test-only `ExecuteMsg::SetResponseMode
//!   { op, mode }` and stored in `ResponseModes` keyed by operation name
//!   (`"open_lease"`, `"close_lease"`, `"swap"`, `"transfer_out"`),
//! - in `Delayed` mode persists the would-be callback (operation name,
//!   sender, payload) into `PendingCallbacks` so the test can advance
//!   blocks and then dispatch via `ExecuteMsg::DeliverPending { op }`.
//!
//! The synthesised responses are realistic-but-fixed:
//!
//! - `OpenLease` → `OperationResponse::OpenLease { remote_lease_id }`
//!   with a synthetic but valid PDA-looking string (the stub mints a fresh
//!   one per `OpenLease` call to mirror Solana's unique-per-lease PDA),
//! - `Swap` → `OperationResponse::Swap { amount_out }`. An asset-direction
//!   swap (the opening legs, which buy the lease currency) pays exactly the
//!   request's configured `min_out` — the literal-floor model confirmed in
//!   plan §10.A.3, which the opening-swap drivers assert against. A
//!   home-direction swap (output in `Lpn` — the repay and liquidation/close
//!   legs that sell the asset for `Lpn`) instead pays a price-derived quote,
//!   the controller-path analogue of the old ICA `do_swap` price callback
//!   (`common/swap.rs::do_swap_internal`). Those legs run `AcceptAnyNonZeroSwap`,
//!   whose `min_out` floors at `1`, so a literal-floor payout would apply
//!   ~1 `Lpn` of proceeds and never settle the operation; pricing the output
//!   off `coin_in` at the harness's identity rate pays the realistic amount,
//! - `TransferOut` → `OperationResponse::TransferOut(TransferOutResponse {})`,
//!   `CloseLease` → `OperationResponse::CloseLease(CloseLeaseResponse {})`.
//!
//! Phase 3-6 of issue #142 wire the lease state machine to actually call
//! these stub methods. The stand-in itself compiles and exercises today
//! against the unchanged callback surface (issue #141 / PR #631).

use serde::{Deserialize, Serialize};

use currencies::{Lpn, PaymentGroup};
use currency::{self, CurrencyDef, Group, MemberOf};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin};
use platform::contract::{Code, CodeId};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
    msg::{CloseLeaseParams, OpenLeaseParams, SwapParams, TransferOutParams},
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId, SwapResponse,
        TransferOutResponse,
    },
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
    cw_storage_plus::{Item, Map},
};
use thiserror::Error;

use super::{ADMIN, CwContractWrapper, test_case::app::App};

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
/// `UnderpayingOnce` answers a swap with `min_out - 1` — a counterparty
/// contract violation — and resets itself to `Ok` so the lease's in-flight
/// retry settles within the same transaction tree. `FailSync` makes the
/// stand-in's execute return an `Err` outright — the outbound submessage
/// reverts (callback never produced, recorders rolled back), modelling a
/// synchronous controller failure rather than a remote one.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    #[default]
    Ok,
    Err(RemoteErrorMessage),
    Delayed,
    UnderpayingOnce,
    FailSync,
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
        params: OpenLeaseParams,
        timeout: finance::duration::Duration,
    },
    CloseLease {
        params: CloseLeaseParams,
        timeout: finance::duration::Duration,
    },
    // #636: `Swap` carries the per-emission `nonce` the lease tags the packet
    // with; the stand-in echoes it into the synthesised callback exactly as the
    // production controller forwards `envelope.nonce`.
    Swap {
        params: SwapParams,
        timeout: finance::duration::Duration,
        #[serde(default)]
        nonce: u64,
    },
    TransferOut {
        params: TransferOutParams,
        timeout: finance::duration::Duration,
    },
    /// Test-only: configure the stand-in's reply for a given op tag.
    SetResponseMode {
        op: String,
        mode: ResponseMode,
    },
    /// Test-only: override the output the next happy-path `Swap` pays,
    /// consumed on use. Lets a driver force a single sell→LPN swap below the
    /// identity quote.
    SetNextSwapOutput {
        amount_out: CoinDTO<PaymentGroup>,
    },
    /// Test-only: dispatch the persisted [`ResponseMode::Delayed`] callback
    /// for the given op tag back to its original sender (the lease).
    DeliverPending {
        op: String,
    },
    /// Test-only: send an arbitrary callback to a lease, exercising
    /// callbacks the lease does not expect in its current state. The
    /// message comes from the stand-in's address, so it passes the lease's
    /// controller authorisation.
    InjectCallback {
        to: Addr,
        callback: RemoteLeaseCallback,
    },
    /// Test-only (#636): send a callback carrying a SPECIFIC `nonce` to a
    /// lease, so a driver can replay the original packet's nonce as a stale
    /// duplicate or deliver a healed re-emission's fresh nonce. Mirrors
    /// [`InjectCallback`] but lets the driver control the per-emission nonce
    /// the production controller would otherwise forward from the envelope.
    InjectCallbackWithNonce {
        to: Addr,
        nonce: u64,
        outcome: RemoteOperationOutcome,
    },
}

/// Stand-in `QueryMsg` — the production variants mirror
/// `remote_lease_controller::api::QueryMsg`; `RecordedSwaps` is additive
/// and test-only.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StubQueryMsg {
    Config(),
    Channel(),
    ProtocolPackageRelease {},
    /// Report every `SwapParams` the given lease has emitted, in order.
    RecordedSwaps {
        lease: Addr,
    },
    /// Report every `TransferOutParams` the given lease has emitted, in order.
    RecordedTransferOuts {
        lease: Addr,
    },
    /// Test-only (#636): report the per-emission `nonce` of every `Swap` the
    /// given lease has emitted, in order, so a driver can replay the in-flight
    /// nonce (matching), the original after a heal (stale), or the healed
    /// re-emission (fresh).
    RecordedSwapNonces {
        lease: Addr,
    },
    /// Report how many `CloseLease` executes the given lease has emitted.
    /// `CloseLeaseParams` is field-less, so a count carries the full record.
    RecordedCloses {
        lease: Addr,
    },
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
const RECORDED_SWAPS: Map<&Addr, Vec<SwapParams>> = Map::new("stub_recorded_swaps");
/// #636: the per-emission nonce each recorded swap went out with, parallel to
/// [`RECORDED_SWAPS`]. Lets a driver replay the in-flight nonce (matching) or
/// the superseded one after a heal (stale).
const RECORDED_SWAP_NONCES: Map<&Addr, Vec<u64>> = Map::new("stub_recorded_swap_nonces");
const RECORDED_TRANSFER_OUTS: Map<&Addr, Vec<TransferOutParams>> =
    Map::new("stub_recorded_transfer_outs");
const RECORDED_CLOSES: Map<&Addr, u32> = Map::new("stub_recorded_closes");
/// A one-shot output override for the next happy-path `Swap`, consumed on
/// use. Set via [`StubExecuteMsg::SetNextSwapOutput`] so a driver can make a
/// single sell→LPN swap pay below the identity quote (e.g. to force a
/// liquidation outcome under the outstanding loan). Absent by default, so the
/// price-derived [`swap_quote`] model is unchanged for every other swap.
const NEXT_SWAP_OUTPUT: Item<CoinDTO<PaymentGroup>> = Item::new("stub_next_swap_output");

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StubConfig {
    connection_id: String,
    dex_label: String,
    transfer_channel: String,
    lease_code: Code,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PendingCallback {
    sender: Addr,
    callback: RemoteLeaseCallback,
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
        transfer_channel: msg.transfer_channel,
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
            handle_outbound(deps, info, op_tag::OPEN_LEASE, 0, |storage| {
                synth_open_lease_response(storage, &params)
            })
        }
        StubExecuteMsg::CloseLease { .. } => {
            record_close(deps.storage, &info.sender)?;
            handle_outbound(deps, info, op_tag::CLOSE_LEASE, 0, |_storage| {
                Ok(OperationResponse::CloseLease(CloseLeaseResponse {}))
            })
        }
        StubExecuteMsg::Swap { params, nonce, .. } => {
            record_swap(deps.storage, &info.sender, &params)?;
            record_swap_nonce(deps.storage, &info.sender, nonce)?;
            handle_outbound(deps, info, op_tag::SWAP, nonce, |storage| {
                Ok(OperationResponse::Swap(SwapResponse {
                    amount_out: next_swap_output(storage, &params)?,
                }))
            })
        }
        StubExecuteMsg::TransferOut { params, .. } => {
            record_transfer_out(deps.storage, &info.sender, &params)?;
            handle_outbound(deps, info, op_tag::TRANSFER_OUT, 0, |_storage| {
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
        StubExecuteMsg::InjectCallback { to, callback } => {
            // #636: a manually-injected "valid" callback must carry the lease's
            // current in-flight nonce so a live swap leg matches it (the leg
            // absorbs any other nonce as `nonce-mismatch`). Stamp the last
            // recorded swap nonce for `to` over the caller's placeholder; if the
            // lease has emitted no swap yet (open/close/transfer-out flows that
            // ignore the nonce), fall back to the caller's value.
            let nonce = last_recorded_swap_nonce(deps.storage, &to)?.unwrap_or(callback.nonce);
            Ok(CwResponse::new().add_message(callback_msg(
                to,
                RemoteLeaseCallback {
                    nonce,
                    outcome: callback.outcome,
                },
            )?))
        }
        StubExecuteMsg::InjectCallbackWithNonce { to, nonce, outcome } => Ok(CwResponse::new()
            .add_message(callback_msg(to, RemoteLeaseCallback { nonce, outcome })?)),
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
                transfer_channel: config.transfer_channel,
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
        StubQueryMsg::RecordedSwaps { lease } => to_json_binary(
            &RECORDED_SWAPS
                .may_load(deps.storage, &lease)?
                .unwrap_or_default(),
        ),
        StubQueryMsg::RecordedTransferOuts { lease } => to_json_binary(
            &RECORDED_TRANSFER_OUTS
                .may_load(deps.storage, &lease)?
                .unwrap_or_default(),
        ),
        StubQueryMsg::RecordedSwapNonces { lease } => to_json_binary(
            &RECORDED_SWAP_NONCES
                .may_load(deps.storage, &lease)?
                .unwrap_or_default(),
        ),
        StubQueryMsg::RecordedCloses { lease } => to_json_binary(
            &RECORDED_CLOSES
                .may_load(deps.storage, &lease)?
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
    require_lease_code(deps.as_ref(), &info, &config)?;

    let mode = MODES
        .may_load(deps.storage, op)?
        .unwrap_or(ResponseMode::Ok);

    // #636: the synthesised callback echoes the emitting packet's `nonce`
    // exactly as the production controller forwards `envelope.nonce`, so a
    // live swap leg credits it instead of absorbing it as `nonce-mismatch`.
    // Non-swap operations pass `0`; their callback handlers ignore the nonce.
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
                    callback: RemoteLeaseCallback {
                        nonce,
                        outcome: payload,
                    },
                },
            )?;
            return Ok(CwResponse::new());
        }
        ResponseMode::UnderpayingOnce => {
            MODES.save(deps.storage, op, &ResponseMode::Ok)?;
            RemoteOperationOutcome::OperationOk(underpay(build_ok(deps.storage)?).into())
        }
        // The Err reverts this whole sub-execution, so the per-op
        // recorders written earlier in the same call roll back with it.
        ResponseMode::FailSync => return Err(StubError::SyncFailure { op: op.to_owned() }),
    };

    Ok(CwResponse::new().add_message(callback_msg(
        info.sender,
        RemoteLeaseCallback { nonce, outcome },
    )?))
}

/// The output a happy-path counterparty pays for a swap.
///
/// An asset-direction swap (the opening legs that buy the lease currency)
/// pays the configured `min_out` floor — the model the opening-swap drivers
/// assert against. A home-direction swap (output in `Lpn`, i.e. the repay
/// and liquidation/close legs that sell the asset back for `Lpn`) instead
/// pays a price-derived quote: the `coin_in` amount re-expressed in the
/// output currency at the harness's identity rate, the controller-path
/// analogue of the old ICA `do_swap` price callback. Those legs run
/// `AcceptAnyNonZeroSwap`, whose floor is `1`, so the literal-floor model
/// would never settle them.
fn swap_quote(params: &SwapParams) -> CoinDTO<PaymentGroup> {
    if params.min_out().currency() == currency::dto::<Lpn, PaymentGroup>() {
        priced_at_identity(params.coin_in().amount(), params.min_out())
    } else {
        *params.min_out()
    }
}

/// The output the next happy-path swap pays.
///
/// A one-shot [`NEXT_SWAP_OUTPUT`] override, if set, is consumed and returned
/// verbatim - letting a driver force a single swap below the identity quote.
/// Absent (the default), the price-derived [`swap_quote`] applies unchanged.
fn next_swap_output(
    storage: &mut dyn Storage,
    params: &SwapParams,
) -> Result<CoinDTO<PaymentGroup>, StubError> {
    match NEXT_SWAP_OUTPUT.may_load(storage)? {
        Some(amount_out) => {
            NEXT_SWAP_OUTPUT.remove(storage);
            Ok(amount_out)
        }
        None => Ok(swap_quote(params)),
    }
}

/// Build a coin of `out`'s currency holding `amount_in` units — the swap
/// output under the harness's identity price (1 input unit ⇒ 1 output unit).
fn priced_at_identity(amount_in: Amount, out: &CoinDTO<PaymentGroup>) -> CoinDTO<PaymentGroup> {
    struct OfAmount(Amount);
    impl WithCoin<PaymentGroup> for OfAmount {
        type Outcome = CoinDTO<PaymentGroup>;

        fn on<C>(self, _coin: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<PaymentGroup> + MemberOf<<PaymentGroup as Group>::TopG>,
        {
            Coin::<C>::new(self.0).into()
        }
    }

    out.with_coin(OfAmount(amount_in))
}

fn underpay(response: OperationResponse) -> OperationResponse {
    match response {
        OperationResponse::Swap(SwapResponse { amount_out }) => {
            OperationResponse::Swap(SwapResponse {
                amount_out: reduce_by_one(&amount_out),
            })
        }
        other => unreachable!("underpaying mode applies to swaps only, got {other:?}"),
    }
}

fn reduce_by_one(coin: &CoinDTO<PaymentGroup>) -> CoinDTO<PaymentGroup> {
    struct SubOne;
    impl WithCoin<PaymentGroup> for SubOne {
        type Outcome = CoinDTO<PaymentGroup>;

        fn on<C>(self, coin: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<PaymentGroup> + MemberOf<<PaymentGroup as Group>::TopG>,
        {
            (coin - Coin::new(1)).into()
        }
    }

    coin.with_coin(SubOne)
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

fn record_swap_nonce(
    storage: &mut dyn Storage,
    sender: &Addr,
    nonce: u64,
) -> Result<(), StubError> {
    RECORDED_SWAP_NONCES
        .update(storage, sender, |recorded| -> Result<_, StdError> {
            let mut recorded = recorded.unwrap_or_default();
            recorded.push(nonce);
            Ok(recorded)
        })
        .map(|_recorded| ())
        .map_err(Into::into)
}

/// #636: the last `nonce` the given lease emitted a swap with, i.e. its
/// current in-flight swap nonce. `None` when the lease has emitted no swap
/// yet (open/close/transfer-out flows that ignore the nonce).
fn last_recorded_swap_nonce(storage: &dyn Storage, lease: &Addr) -> Result<Option<u64>, StubError> {
    Ok(RECORDED_SWAP_NONCES
        .may_load(storage, lease)?
        .and_then(|nonces| nonces.last().copied()))
}

fn record_close(storage: &mut dyn Storage, sender: &Addr) -> Result<(), StubError> {
    RECORDED_CLOSES
        .update(storage, sender, |recorded| -> Result<_, StdError> {
            Ok(recorded.unwrap_or_default() + 1)
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
    _params: &OpenLeaseParams,
) -> Result<OperationResponse, StubError> {
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

fn deliver_pending(storage: &mut dyn Storage, op: &str) -> Result<CwResponse, StubError> {
    let pending = PENDING
        .may_load(storage, op)?
        .ok_or_else(|| StubError::NoPending { op: op.to_owned() })?;
    PENDING.remove(storage, op);
    Ok(CwResponse::new().add_message(callback_msg(pending.sender, pending.callback)?))
}

fn callback_msg(lease: Addr, callback: RemoteLeaseCallback) -> StdResult<WasmMsg> {
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
            transfer_channel: "channel-0".into(),
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

/// Trigger delivery of a previously stored Delayed callback for the
/// given op tag.
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

/// Deliver an arbitrary callback to a lease from the stand-in's
/// (authorised) address.
pub fn inject_callback(
    app: &mut App,
    controller: &Addr,
    lease: &Addr,
    callback: RemoteLeaseCallback,
) -> sdk::cw_multi_test::AppResponse {
    let msg = StubExecuteMsg::InjectCallback {
        to: lease.clone(),
        callback,
    };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| response.unwrap_response())
        .expect("InjectCallback must succeed against the stand-in")
}

/// Deliver a callback carrying a SPECIFIC nonce to a lease from the stand-in's
/// (authorised) address (#636). Lets a driver replay the original packet's
/// nonce as a stale duplicate, or deliver a healed re-emission's fresh nonce.
pub fn inject_callback_with_nonce(
    app: &mut App,
    controller: &Addr,
    lease: &Addr,
    nonce: u64,
    outcome: RemoteOperationOutcome,
) -> sdk::cw_multi_test::AppResponse {
    let msg = StubExecuteMsg::InjectCallbackWithNonce {
        to: lease.clone(),
        nonce,
        outcome,
    };
    app.execute(sdk::testing::user(ADMIN), controller.clone(), &msg, &[])
        .map(|response| response.unwrap_response())
        .expect("InjectCallbackWithNonce must succeed against the stand-in")
}

/// Report every `SwapParams` the given lease has emitted.
pub fn recorded_swaps(app: &App, controller: &Addr, lease: &Addr) -> Vec<SwapParams> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedSwaps {
                lease: lease.clone(),
            },
        )
        .expect("RecordedSwaps must succeed against the stand-in")
}

/// Report the per-emission nonce of every `Swap` the given lease has emitted
/// (#636), parallel to [`recorded_swaps`].
pub fn recorded_swap_nonces(app: &App, controller: &Addr, lease: &Addr) -> Vec<u64> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedSwapNonces {
                lease: lease.clone(),
            },
        )
        .expect("RecordedSwapNonces must succeed against the stand-in")
}

/// Report every `TransferOutParams` the given lease has emitted.
pub fn recorded_transfer_outs(
    app: &App,
    controller: &Addr,
    lease: &Addr,
) -> Vec<TransferOutParams> {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedTransferOuts {
                lease: lease.clone(),
            },
        )
        .expect("RecordedTransferOuts must succeed against the stand-in")
}

/// Report how many `CloseLease` executes the given lease has emitted.
pub fn recorded_closes(app: &App, controller: &Addr, lease: &Addr) -> u32 {
    app.query()
        .query_wasm_smart(
            controller.clone(),
            &StubQueryMsg::RecordedCloses {
                lease: lease.clone(),
            },
        )
        .expect("RecordedCloses must succeed against the stand-in")
}
