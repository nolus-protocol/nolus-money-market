use serde::{Deserialize, Serialize};

use currency::{lpn::Lpns, SymbolSlice};
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, SwapState, SwapTask,
    TransferInFinishState, TransferInInitState, TransferOutState,
};
use finance::coin::CoinDTO;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, opened::PositionCloseTrx},
    contract::{
        state::{opened::payment::Repayable, SwapResult},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::Closable;

type SellAssetStateResponse<RepayableT> = <SellAsset<RepayableT> as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub(crate) struct SellAsset<RepayableT> {
    lease: Lease,
    repayable: RepayableT,
}

impl<RepayableT> SellAsset<RepayableT> {
    pub(in crate::contract::state) fn new(lease: Lease, repayable: RepayableT) -> Self {
        Self { lease, repayable }
    }
}

impl<RepayableT> SwapTask for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    type OutG = Lpns;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        self.repayable.event_type()
    }

    fn dex_account(&self) -> &Account {
        &self.lease.dex
    }

    fn oracle(&self) -> &OracleRef {
        &self.lease.lease.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    fn out_currency(&self) -> &SymbolSlice {
        self.lease.lease.loan.lpp().currency()
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        dex::on_coin(self.repayable.amount(&self.lease), visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        self.repayable
            .try_repay(self.lease, amount_out, env, querier)
    }
}

impl<RepayableT> ContractInSwap<TransferOutState, SellAssetStateResponse<RepayableT>>
    for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> SellAssetStateResponse<RepayableT> {
        // it's due to reusing the same enum dex::State
        // have to define a tailored enum dex::State that starts from SwapExactIn
        unreachable!(
            "The sell lease asset on liquidation task never goes through a 'TransferOut' state!"
        )
    }
}

impl<RepayableT> ContractInSwap<SwapState, SellAssetStateResponse<RepayableT>>
    for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    fn state(
        self,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> SellAssetStateResponse<RepayableT> {
        super::query(
            self.lease,
            self.repayable,
            PositionCloseTrx::Swap,
            now,
            querier,
        )
    }
}

impl<RepayableT> ContractInSwap<TransferInInitState, SellAssetStateResponse<RepayableT>>
    for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    fn state(
        self,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> SellAssetStateResponse<RepayableT> {
        super::query(
            self.lease,
            self.repayable,
            PositionCloseTrx::TransferInInit,
            now,
            querier,
        )
    }
}

impl<RepayableT> ContractInSwap<TransferInFinishState, SellAssetStateResponse<RepayableT>>
    for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    fn state(
        self,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> SellAssetStateResponse<RepayableT> {
        super::query(
            self.lease,
            self.repayable,
            PositionCloseTrx::TransferInFinish,
            now,
            querier,
        )
    }
}
