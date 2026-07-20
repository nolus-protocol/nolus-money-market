use serde::{Deserialize, Serialize};

use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyTreatment, ContractInSwap, Error as DexError, Stage,
    StartLocalLocalState, SwapCoins, SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
};
use finance::instant::Instant;
use finance::{coin::Coin, duration::Duration};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::contract::transport::{SwapClientFactory, TransferOutFactory};
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
            StateResponse as ContractStateResponse, SwapResult,
            opened::{self, repay},
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LpnCurrencies, LpnCurrency},
};

pub(super) type StartState =
    StartLocalLocalState<BuyLpn, TransferOutFactory, SwapClientFactory, ForwardToDexEntry>;
pub(crate) type DexState =
    dex::StateLocalOut<BuyLpn, TransferOutFactory, SwapClientFactory, ForwardToDexEntry>;

pub(in super::super) fn start(lease: Lease, payment: PaymentCoin) -> StartState {
    dex::start_local_local(
        BuyLpn::new(lease, payment),
        TransferOutFactory::default(),
        SwapClientFactory::default(),
    )
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}

impl BuyLpn {
    fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
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

    fn coins(&self) -> SwapCoins<Self::InG> {
        SwapCoins::One(self.payment)
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
        // TODO repay with Coin, not CoinDTO
        repay::repay(self.lease, amount_out.into(), env, querier)
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
        self.query(in_progress.into(), now, due_projection, querier)
    }
}

impl From<Stage> for RepayTrx {
    fn from(value: Stage) -> Self {
        match value {
            Stage::TransferOut => Self::TransferOut,
            Stage::Swap => Self::Swap,
            Stage::TransferInInit => Self::TransferInInit,
            Stage::TransferInFinish => Self::TransferInFinish,
        }
    }
}
