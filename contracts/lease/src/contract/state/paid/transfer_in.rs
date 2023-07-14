use serde::{Deserialize, Serialize};

use currency::{lease::LeaseGroup, Symbol};
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, StartTransferInState, SwapState,
    SwapTask, TransferInFinishState, TransferInInitState, TransferOutState,
};
use finance::coin::CoinDTO;
use oracle::stub::OracleRef;
use platform::{
    message::Response as MessageResponse, state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, paid::ClosingTrx, StateResponse},
    contract::{
        state::{closed::Closed, resp_delivery::ForwardToDexEntry, SwapResult},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

type AssetGroup = LeaseGroup;
pub(super) type StartState = StartTransferInState<TransferIn, ForwardToDexEntry>;
pub(in crate::contract::state) type DexState = dex::StateLocalOut<TransferIn, ForwardToDexEntry>;

pub(in crate::contract::state) fn start(lease: Lease) -> StartState {
    let amount_in = lease.lease.amount.clone();
    StartState::new(TransferIn::new(lease), amount_in)
}

type TransferInState = <TransferIn as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub(crate) struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    fn state(self, in_progress: ClosingTrx) -> <Self as SwapTask>::StateResponse {
        Ok(StateResponse::paid_from(
            self.lease.lease,
            Some(in_progress),
        ))
    }

    // fn emit_ok(&self) -> Emitter {
    //     Emitter::of_type(Type::OpeningTransferOut)
    // }
}

impl SwapTask for TransferIn {
    type OutG = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::ClosingTransferIn
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

    fn out_currency(&self) -> Symbol<'_> {
        self.lease.lease.amount.ticker()
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        dex::on_coin(&self.lease.lease.amount, visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        debug_assert!(amount_out == self.lease.lease.amount);
        let closed = Closed::default();
        let emitter = closed.emit_ok(env, &self.lease.lease);
        let batch = closed.enter_state(self.lease.lease, querier)?;
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_events(batch, emitter),
            closed,
        ))
    }
}

impl ContractInSwap<TransferOutState, TransferInState> for TransferIn {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> TransferInState {
        // it's due to reusing the same enum dex::State
        // have to define a tailored enum dex::State that starts from TransferIn
        unreachable!("The lease asset transfer-in task never goes through a 'TransferOut' state!")
    }
}

impl ContractInSwap<SwapState, TransferInState> for TransferIn {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> TransferInState {
        // it's due to reusing the same enum dex::State
        // have to define a tailored enum dex::State that starts from TransferIn
        unreachable!("The lease asset transfer-in task never goes through a 'Swap'!")
    }
}

impl ContractInSwap<TransferInInitState, TransferInState> for TransferIn {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> TransferInState {
        self.state(ClosingTrx::TransferInInit)
    }
}

impl ContractInSwap<TransferInFinishState, TransferInState> for TransferIn {
    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> TransferInState {
        self.state(ClosingTrx::TransferInFinish)
    }
}
