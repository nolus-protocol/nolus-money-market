use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef};
use dex::{Account, CoinVisitor, ContractInSwap, IterNext, IterState, Stage, SwapTask};
use finance::{coin::CoinDTO, duration::Duration};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeaseAssetCurrencies, LeasePaymentCurrencies,
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

    pub(super) fn drop(self) -> (Lease, RepayableT) {
        (self.lease, self.repayable)
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
    type InOutG = LeasePaymentCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        self.repayable.event_type()
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

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<GIn = Self::InG, Result = IterNext>,
    {
        dex::on_coin(self.repayable.amount(&self.lease), visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        self.repayable
            .try_repay(self.lease, amount_out, env, querier)
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
