use std::iter;

use currency::Group;
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::{
    Account, AnomalyTreatment, ContractInSwap, SlippageCalculator, Stage, SwapOutputTask, SwapTask,
    WithCalculator, WithOutputTask,
};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
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
            SwapClient, SwapResult,
            opened::{self, payment::Repayable},
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::ContractResult,
    event::Type,
    finance::LpnCurrencies,
};

use super::{AnomalyHandler, Calculator, Closable};

pub(in crate::contract::state) mod customer_close;
pub(in crate::contract::state) mod liquidation;
mod migrate_v0_8_7;
mod task;

type Task<RepayableT, CalculatorT> = SellAsset<RepayableT, CalculatorT>;
type DexState<Repayable, CalculatorT> =
    dex::StateLocalOut<Task<Repayable, CalculatorT>, SwapClient, ForwardToDexEntry>;

#[derive(Serialize, Deserialize)]
pub(crate) struct SellAsset<RepayableT, CalculatorT>
where
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
{
    lease: Lease,
    repayable: RepayableT,
    #[serde(default = "CalculatorT::default")] //TODO remove past the migration from v0.8.7
    slippage_calc: CalculatorT,
}

impl<RepayableT, CalculatorT> SellAsset<RepayableT, CalculatorT>
where
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
{
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
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
{
    fn query(
        self,
        in_progress: PositionCloseTrx,
        now: Timestamp,
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
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
    CalculatorT: Calculator,
    Self: AnomalyHandler<Self>,
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
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
    CalculatorT: Calculator,
    Self: AnomalyHandler<Self>,
{
    type OutC = <CalculatorT as SlippageCalculator<<Self as SwapTask>::InG>>::OutC;

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
        <Self as AnomalyHandler<Self>>::on_anomaly(self)
    }

    fn finish(
        self,
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <Self as SwapTask>::Result {
        // TODO repay with Coin, not CoinDTO
        self.repayable
            .try_repay(self.lease, amount_out.into(), env, querier)
    }
}

impl<RepayableT, CalculatorT> ContractInSwap for SellAsset<RepayableT, CalculatorT>
where
    RepayableT: Closable + Repayable,
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
    CalculatorT: Calculator,
    Self: AnomalyHandler<Self>,
{
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

impl From<Stage> for PositionCloseTrx {
    fn from(value: Stage) -> Self {
        match value {
            Stage::TransferOut => unreachable!(
                "The sell lease asset on liquidation task never goes through a 'TransferOut' state!"
            ),
            Stage::Swap => Self::Swap,
            Stage::TransferInInit => Self::TransferInInit,
            Stage::TransferInFinish => Self::TransferInFinish,
        }
    }
}
