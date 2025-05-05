use std::iter;

use currency::{CurrencyDTO, CurrencyDef, Group};
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::{Account, ContractInSwap, Stage, SwapOutputTask, SwapTask, WithOutputTask};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeaseAssetCurrencies,
        query::{StateResponse as QueryStateResponse, opened::PositionCloseTrx},
    },
    contract::{
        Lease,
        state::{
            SwapResult,
            opened::{self, payment::Repayable},
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LpnCurrencies, LpnCurrency},
};

use super::Closable;

#[derive(Serialize, Deserialize)]
pub(crate) struct SellAsset<RepayableT> {
    lease: Lease,
    repayable: RepayableT,
}

impl<RepayableT> SellAsset<RepayableT> {
    pub(in super::super) fn new(lease: Lease, repayable: RepayableT) -> Self {
        Self { lease, repayable }
    }
}

impl<RepayableT> SellAsset<RepayableT>
where
    RepayableT: Closable,
{
    fn query(
        self,
        in_progress: PositionCloseTrx,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        let trx = self.repayable.transaction(&self.lease, in_progress);
        opened::lease_state(self.lease, Some(trx), now, due_projection, querier)
    }
}

impl<RepayableT> SwapTask for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
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

    fn out_currency(&self) -> CurrencyDTO<Self::OutG> {
        *LpnCurrency::dto()
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        iter::once(*self.repayable.amount(&self.lease))
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        cmd.on(self)
    }
}

impl<RepayableT> SwapOutputTask<Self> for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    type OutC = LpnCurrency;

    fn as_spec(&self) -> &Self {
        self
    }

    fn into_spec(self) -> Self {
        self
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

impl<RepayableT> ContractInSwap for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
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
