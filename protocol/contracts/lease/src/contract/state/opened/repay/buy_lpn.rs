use serde::{Deserialize, Serialize};

use currencies::Lpns;
use currency::SymbolSlice;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, StartLocalLocalState, SwapState,
    SwapTask, TransferInFinishState, TransferInInitState, TransferOutState,
};
use finance::coin::CoinDTO;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        query::{
            opened::{OngoingTrx, RepayTrx},
            StateResponse as QueryStateResponse,
        },
        PaymentCoin,
    },
    contract::{
        state::{
            opened::{self, repay},
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            StateResponse as ContractStateResponse, SwapResult,
        },
        Lease,
    },
    error::ContractResult,
    event::Type,
};

pub(super) type StartState =
    StartLocalLocalState<BuyLpn, ForwardToDexEntry, ForwardToDexEntryContinue>;
pub(crate) type DexState = dex::StateLocalOut<BuyLpn, ForwardToDexEntry, ForwardToDexEntryContinue>;

pub(in crate::contract::state) fn start(lease: Lease, payment: PaymentCoin) -> StartState {
    dex::start_local_local(BuyLpn::new(lease, payment))
}

type BuyLpnStateResponse = <BuyLpn as SwapTask>::StateResponse;

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
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<ContractStateResponse> {
        let in_progress = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress,
        };

        opened::lease_state(self.lease, Some(in_progress), now, querier)
    }
}

impl SwapTask for BuyLpn {
    type OutG = Lpns;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::RepaymentSwap
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
        dex::on_coin(&self.payment, visitor)
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

impl<DexState> ContractInSwap<DexState, BuyLpnStateResponse> for BuyLpn
where
    DexState: InProgressTrx,
{
    fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> BuyLpnStateResponse {
        self.query(DexState::trx_in_progress(), now, querier)
    }
}

trait InProgressTrx {
    fn trx_in_progress() -> RepayTrx;
}

impl InProgressTrx for TransferOutState {
    fn trx_in_progress() -> RepayTrx {
        RepayTrx::TransferOut
    }
}

impl InProgressTrx for SwapState {
    fn trx_in_progress() -> RepayTrx {
        RepayTrx::Swap
    }
}

impl InProgressTrx for TransferInInitState {
    fn trx_in_progress() -> RepayTrx {
        RepayTrx::TransferInInit
    }
}

impl InProgressTrx for TransferInFinishState {
    fn trx_in_progress() -> RepayTrx {
        RepayTrx::TransferInFinish
    }
}
