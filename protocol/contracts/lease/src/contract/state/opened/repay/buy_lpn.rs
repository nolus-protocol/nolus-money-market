use std::iter;

use currency::Group;
use cw_time::IntoInstant;
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyTreatment, CoinsNb, Connectable, ContractInRemoteSwap,
    ContractInSwap, Enterable, Error as DexError, FundingClient, RemoteSwapClient,
    SlippageEscalation, Stage, SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
};
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use platform::{batch::Batch, ica::HostAccount};
use remote_lease::{
    msg::SwapParams,
    response::{OperationResponse, SwapResponse},
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeasePaymentCurrencies, PaymentCoin,
        query::{
            StateResponse as QueryStateResponse,
            opened::{OngoingTrx, RepayTrx, Status},
        },
    },
    contract::{
        Lease,
        state::{
            Response, StateResponse as ContractStateResponse, SwapResult,
            opened::{
                self,
                proceeds_drain::{RepayDrain, RepayFinish},
            },
            remote_lease_host,
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::{ContractError, ContractResult},
    event::Type,
    finance::{LpnCurrencies, LpnCurrency},
};

/// A non-`Swap` success acknowledgment can only come from a buggy or
/// hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_SWAP_RESPONSE: &str = "non-swap operation response";

/// The acknowledged output currency is not the lease's LPN, so the
/// response cannot have originated from the scheduled repay swap.
const OUT_NOT_LPN: &str = "swapped-out currency is not the lease LPN";

const TIMEOUT_RETRY_BUDGET: CoinsNb = 3;

pub(crate) type DexState = dex::StateFundRemote<BuyLpn, ForwardToDexEntry>;
pub(crate) type DrainState = dex::StateDrain<RepayDrain<RepayFinish>>;

pub(in super::super) fn start(
    lease: Lease,
    payment: PaymentCoin,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> SwapResult {
    let start_state = BuyLpn::new(lease, payment).and_then(|spec| {
        dex::start_fund_remote::<_, ForwardToDexEntry>(spec).map_err(Into::into)
    })?;
    start_state
        .enter(env.block.time.into_instant(), querier)
        .map(|funding_msgs| Response::from(funding_msgs, DexState::from(start_state)))
        .map_err(Into::into)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
    /// The `LeaseAuthority` the repay funding transfer is addressed to, bridged
    /// from the lease's persisted `remote_lease_id` so it can be lent as a
    /// `&HostAccount` to the ICS-20 sender — exactly as the opening funds the
    /// downpayment and principal.
    funding_receiver: HostAccount,
}

impl BuyLpn {
    fn new(lease: Lease, payment: PaymentCoin) -> ContractResult<Self> {
        remote_lease_host(&lease.lease.remote_lease_id).map(|funding_receiver| Self {
            lease,
            payment,
            funding_receiver,
        })
    }

    fn query(
        self,
        in_progress: RepayTrx,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<ContractStateResponse> {
        let in_progress = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress,
        };

        opened::lease_state(
            self.lease,
            Status::InProgress(in_progress),
            now,
            due_projection,
            querier,
        )
    }
}

impl SwapTask for BuyLpn {
    type InG = LeasePaymentCurrencies;
    type OutG = LpnCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::RepaymentSwap
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
        TIMEOUT_RETRY_BUDGET
    }

    fn slippage_escalation(&self) -> SlippageEscalation {
        SlippageEscalation::Park
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        iter::once(self.payment)
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        with_calc.on(&AcceptAnyNonZeroSwap::<
            _,
            <Self as SwapOutputTask<Self>>::OutC,
        >::default())
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        cmd.on(self)
    }
}

impl SwapOutputTask<Self> for BuyLpn {
    type OutC = LpnCurrency;

    fn as_spec(&self) -> &Self {
        self
    }

    fn into_spec(self) -> Self {
        self
    }

    fn on_anomaly(self) -> AnomalyTreatment<Self>
    where
        Self: Sized,
    {
        AnomalyTreatment::Retry(self)
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
            RepayFinish::new(self.payment),
        );
        dex::start_drain(drain)
            .and_then(|start_drain| {
                start_drain
                    .enter(env.block.time.into_instant(), querier)
                    .map(|drain_msgs| Response::from(drain_msgs, DrainState::from(start_drain)))
            })
            .map_err(Into::into)
    }
}

impl RemoteSwapClient for BuyLpn {
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
        nonce: u64,
    ) -> dex::DexResult<Batch> {
        SwapParams::new(*coin_in, min_out.into_super_group())
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

    /// The repay swap parks a zero-acked error rather than unwinding: a repay
    /// has a live lease to recover into, so it does not opt into the opening's
    /// drain-home unwind (`unwind_on_zero_acked` keeps its `false` default).
    /// This path is therefore unreachable; it returns a visible error rather
    /// than driving an unwind the repay flow has no inputs to drain.
    fn unwind(self, _querier: QuerierWrapper<'_>, _env: &Env) -> <Self as SwapTask>::Result {
        Err(ContractError::unsupported_operation(
            "repay swap does not unwind on a zero-acked error",
        ))
    }
}

impl FundingClient for BuyLpn {
    fn funding_sender(&self) -> &Addr {
        self.dex_account().owner()
    }

    fn funding_receiver(&self) -> &HostAccount {
        &self.funding_receiver
    }

    fn transfer_channel(&self) -> &str {
        self.dex_account()
            .dex()
            .transfer_channel
            .local_endpoint
            .as_str()
    }
}

impl ContractInSwap for BuyLpn {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        match in_progress {
            Stage::TransferOut => self.query(RepayTrx::TransferOut, now, due_projection, querier),
            Stage::Swap => unimplemented!("the repay swap runs over the remote-lease transport"),
            Stage::TransferInInit => unimplemented!(),
            Stage::TransferInFinish => unimplemented!(),
        }
    }
}

impl ContractInRemoteSwap for BuyLpn {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        _acks_left: CoinsNb,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.query(RepayTrx::Swap, now, due_projection, querier)
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
