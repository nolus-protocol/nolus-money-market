use std::iter;

use currency::Group;
use cw_time::IntoInstant;
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::{
    Account, CoinsNb, ContractInRemoteSwap, Enterable, Error as DexError, RemoteSwapClient,
    SlippageCalculator, SlippageEscalation, SwapOutputTask, SwapTask, WithCalculator,
    WithOutputTask,
};
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use platform::batch::Batch;
use remote_lease::{
    msg::SwapParams,
    response::{OperationResponse, SwapResponse},
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeaseAssetCurrencies,
        query::{
            StateResponse as QueryStateResponse,
            opened::{PositionCloseTrx, Status},
        },
    },
    contract::{
        Lease,
        state::{
            Response, State, SwapResult,
            opened::{
                self,
                payment::Repayable,
                proceeds_drain::{CloseFinish, RepayDrain},
            },
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LpnCurrencies, LpnCurrency},
};

use super::{Calculator, Closable};

pub(in crate::contract::state) mod customer_close;
pub(in crate::contract::state) mod liquidation;
mod task;

/// A non-`Swap` success acknowledgment can only come from a buggy or
/// hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_SWAP_RESPONSE: &str = "non-swap operation response";

/// The acknowledged output currency is not the lease's LPN, so the
/// response cannot have originated from the scheduled close swap.
const OUT_NOT_LPN: &str = "swapped-out currency is not the lease LPN";

type Task<RepayableT, CalculatorT> = SellAsset<RepayableT, CalculatorT>;
pub(crate) type DexState<Repayable, CalculatorT> = dex::StateSwap<Task<Repayable, CalculatorT>>;
pub(crate) type DrainState<Repayable> = dex::StateDrain<RepayDrain<CloseFinish<Repayable>>>;

#[derive(Serialize, Deserialize)]
pub(crate) struct SellAsset<RepayableT, CalculatorT> {
    lease: Lease,
    repayable: RepayableT,
    slippage_calc: CalculatorT,
}

impl<RepayableT, CalculatorT> SellAsset<RepayableT, CalculatorT> {
    pub(in super::super) fn new(
        lease: Lease,
        repayable: RepayableT,
        slippage_calc: CalculatorT,
    ) -> Self {
        Self {
            lease,
            repayable,
            slippage_calc,
        }
    }
}

impl<RepayableT, CalculatorT> SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable,
{
    fn query(
        self,
        in_progress: PositionCloseTrx,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        let trx = self.repayable.transaction(&self.lease, in_progress);
        opened::lease_state(
            self.lease,
            Status::InProgress(trx),
            now,
            due_projection,
            querier,
        )
    }
}

impl<RepayableT, CalculatorT> SwapTask for SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable + Repayable,
    CalculatorT: Calculator,
    DrainState<RepayableT>: Into<State>,
{
    type InG = LeaseAssetCurrencies;
    type OutG = LpnCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        self.repayable.event_type()
    }

    fn dex_account(&self) -> &Account {
        &self.lease.dex
    }

    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
        &self.lease.lease.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(
            &self.lease.leases.remote_lease_callback_permission(querier),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn authz_anomaly_resolution(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(
            &self.lease.leases.anomaly_resolution_permission(querier),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn timeout_retry_budget(&self) -> CoinsNb {
        self.repayable.timeout_retry_budget()
    }

    fn slippage_escalation(&self) -> SlippageEscalation {
        SlippageEscalation::Park
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        iter::once(*self.repayable.amount(&self.lease))
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        with_calc.on(&self.slippage_calc)
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        cmd.on(self)
    }
}

impl<RepayableT, CalculatorT> SwapOutputTask<Self> for SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable + Repayable,
    CalculatorT: Calculator,
    DrainState<RepayableT>: Into<State>,
{
    type OutC = <CalculatorT as SlippageCalculator<<Self as SwapTask>::InG>>::OutC;

    fn as_spec(&self) -> &Self {
        self
    }

    fn into_spec(self) -> Self {
        self
    }

    fn on_anomaly(self) -> dex::AnomalyTreatment<Self>
    where
        Self: Sized,
    {
        unreachable!(
            "the swap-only composite re-emits the in-flight leg on an anomaly and never delegates to `on_anomaly`"
        )
    }

    fn finish(
        self,
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <Self as SwapTask>::Result {
        let drain = RepayDrain::new(
            self.lease,
            amount_out.into(),
            CloseFinish::new(self.repayable),
        );
        dex::start_drain(drain)
            .and_then(|start_drain| {
                start_drain
                    .enter(env.block.time.into_instant(), querier)
                    .map(|drain_msgs| {
                        Response::from(drain_msgs, DrainState::<RepayableT>::from(start_drain))
                    })
            })
            .map_err(Into::into)
    }
}

impl<RepayableT, CalculatorT> RemoteSwapClient for SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable + Repayable,
    CalculatorT: Calculator,
    DrainState<RepayableT>: Into<State>,
{
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
        nonce: u64,
    ) -> dex::DexResult<Batch> {
        SwapParams::new(coin_in.into_super_group(), min_out.into_super_group())
            .map_err(DexError::remote_swap_client)
            .and_then(|params| {
                ControllerLease::new(&self.lease.lease.remote_lease_controller)
                    .swap(params, SwapParams::TIMEOUT, |params, timeout| {
                        ControllerExecuteMsg::Swap {
                            params,
                            timeout,
                            nonce,
                        }
                    })
                    .map_err(Into::into)
            })
    }

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<CoinDTO<Self::OutG>> {
        cosmwasm_std::from_json::<OperationResponse>(payload)
            .map_err(DexError::remote_swap_client)
            .and_then(|response| match response {
                OperationResponse::Swap(SwapResponse { amount_out }) => {
                    Coin::<LpnCurrency>::try_from(amount_out)
                        .map(Into::into)
                        .map_err(|_not_lpn| DexError::unexpected_response_variant(OUT_NOT_LPN))
                }
                OperationResponse::OpenLease(_)
                | OperationResponse::CloseLease(_)
                | OperationResponse::TransferOut(_) => {
                    Err(DexError::unexpected_response_variant(NON_SWAP_RESPONSE))
                }
            })
    }
}

impl<RepayableT, CalculatorT> ContractInRemoteSwap for SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable + Repayable,
    CalculatorT: Calculator,
    DrainState<RepayableT>: Into<State>,
{
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        _acks_left: CoinsNb,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.query(PositionCloseTrx::Swap, now, due_projection, querier)
    }

    fn anomaly_response(
        self,
        _acks_left: CoinsNb,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        opened::lease_state(
            self.lease,
            Status::SlippageProtectionActivated,
            now,
            due_projection,
            querier,
        )
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    Swap {
        params: SwapParams,
        timeout: Duration,
        nonce: u64,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}
