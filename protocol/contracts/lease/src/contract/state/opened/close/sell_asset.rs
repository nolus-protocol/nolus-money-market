use serde::{Deserialize, Serialize};

use currency::SymbolSlice;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, SwapState, SwapTask,
    TransferInFinishState, TransferInInitState, TransferOutState,
};
use finance::coin::CoinDTO;
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::query::{opened::PositionCloseTrx, StateResponse as QueryStateResponse},
    contract::{
        state::{
            opened::{self, payment::Repayable},
            SwapResult,
        },
        Lease,
    },
    error::ContractResult,
    event::Type,
    finance::{LpnCurrencies, OracleRef},
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
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        let trx = self.repayable.transaction(&self.lease, in_progress);
        opened::lease_state(self.lease, Some(trx), now, querier)
    }
}

impl<RepayableT> SwapTask for SellAsset<RepayableT>
where
    RepayableT: Closable + Repayable,
{
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

    fn oracle(&self) -> &OracleRef {
        &self.lease.lease.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.lease.lease.time_alarms
    }

    fn out_currency(&self) -> &SymbolSlice {
        self.lease.lease.loan.lpp().lpn()
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
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        self.repayable
            .try_repay(self.lease, amount_out, env, querier)
    }
}

impl<DexState, RepayableT> ContractInSwap<DexState> for SellAsset<RepayableT>
where
    DexState: InProgressTrx,
    RepayableT: Closable + Repayable,
{
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> Self::StateResponse {
        self.query(DexState::trx_in_progress(), now, querier)
    }
}

trait InProgressTrx {
    fn trx_in_progress() -> PositionCloseTrx;
}

impl InProgressTrx for TransferOutState {
    fn trx_in_progress() -> PositionCloseTrx {
        // it's due to reusing the same enum dex::State
        // have to define a tailored enum dex::State that starts from SwapExactIn
        unreachable!(
            "The sell lease asset on liquidation task never goes through a 'TransferOut' state!"
        )
    }
}

impl InProgressTrx for SwapState {
    fn trx_in_progress() -> PositionCloseTrx {
        PositionCloseTrx::Swap
    }
}

impl InProgressTrx for TransferInInitState {
    fn trx_in_progress() -> PositionCloseTrx {
        PositionCloseTrx::TransferInInit
    }
}

impl InProgressTrx for TransferInFinishState {
    fn trx_in_progress() -> PositionCloseTrx {
        PositionCloseTrx::TransferInFinish
    }
}
