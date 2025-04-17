use std::iter;

use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef};
use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyMonitoredTask, AnomalyPolicy, ContractInSwap, Stage,
    StartLocalLocalState, SwapTask,
};
use finance::{coin::CoinDTO, duration::Duration};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeasePaymentCurrencies, PaymentCoin,
        query::{
            StateResponse as QueryStateResponse,
            opened::{OngoingTrx, RepayTrx},
        },
    },
    contract::{
        Lease,
        state::{
            StateResponse as ContractStateResponse, SwapClient, SwapResult,
            opened::{self, repay},
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LpnCurrencies, LpnCurrency},
};

pub(super) type StartState =
    StartLocalLocalState<BuyLpn, LeasePaymentCurrencies, SwapClient, ForwardToDexEntry>;
pub(crate) type DexState =
    dex::StateLocalOut<BuyLpn, LeasePaymentCurrencies, SwapClient, ForwardToDexEntry>;

pub(in super::super) fn start(lease: Lease, payment: PaymentCoin) -> StartState {
    dex::start_local_local(BuyLpn::new(lease, payment))
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
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<ContractStateResponse> {
        let in_progress = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress,
        };

        opened::lease_state(self.lease, Some(in_progress), now, due_projection, querier)
    }
}

impl SwapTask for BuyLpn {
    type InG = LeasePaymentCurrencies;
    type OutG = LpnCurrencies;
    type InOutG = LeasePaymentCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::RepaymentSwap
    }

    fn dex_account(&self) -> &Account {
        &self.lease.dex
    }

    fn oracle(&self) -> &impl SwapPath<Self::InOutG> {
        &self.lease.lease.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    fn out_currency(&self) -> CurrencyDTO<Self::OutG> {
        *LpnCurrency::dto()
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        iter::once(self.payment)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        repay::repay(self.lease, amount_out, env, querier)
    }
}

impl AnomalyMonitoredTask for BuyLpn {
    fn policy(&self) -> impl AnomalyPolicy<Self> {
        AcceptAnyNonZeroSwap::on_task(self)
    }
}

impl ContractInSwap for BuyLpn {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        now: Timestamp,
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
