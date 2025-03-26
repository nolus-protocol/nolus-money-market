use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::CurrencyDTO;
use dex::{
    Account, CoinVisitor, ContractInSwap, IterNext, IterState, Stage, StartTransferInState,
    SwapTask,
};
use finance::{coin::CoinDTO, duration::Duration};
use platform::{
    bank,
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        LeaseAssetCurrencies, LeasePaymentCurrencies,
        query::{StateResponse as QueryStateResponse, paid::ClosingTrx},
    },
    contract::{
        Lease,
        cmd::Close,
        state::{SwapClient, SwapResult, closed::Closed, resp_delivery::ForwardToDexEntry},
    },
    error::ContractResult,
    event::Type,
    lease::{LeaseDTO, with_lease_paid},
};

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState =
    StartTransferInState<TransferIn, LeasePaymentCurrencies, SwapClient, ForwardToDexEntry>;
pub(in super::super) type DexState =
    dex::StateLocalOut<TransferIn, LeasePaymentCurrencies, SwapClient, ForwardToDexEntry>;

pub(in super::super) fn start(lease: Lease) -> StartState {
    let transfer = TransferIn::new(lease);
    let amount_in = *transfer.amount();
    StartState::new(transfer, amount_in)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(in super::super) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    fn state(self, in_progress: ClosingTrx) -> <Self as SwapTask>::StateResponse {
        Ok(QueryStateResponse::paid_from(
            self.lease.lease,
            Some(in_progress),
        ))
    }

    fn amount(&self) -> &CoinDTO<LeaseAssetCurrencies> {
        self.lease.lease.position.amount()
    }

    fn emit_ok(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl SwapTask for TransferIn {
    type InG = AssetGroup;
    type OutG = AssetGroup;
    type InOutG = LeasePaymentCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::ClosingTransferIn
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
        self.amount().currency()
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<GIn = Self::InG, Result = IterNext>,
    {
        dex::on_coin(self.amount(), visitor)
    }

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        debug_assert!(&amount_out == self.amount());
        let lease_addr = self.lease.lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        let emitter = self.emit_ok(env, &self.lease.lease);
        let customer = self.lease.lease.customer.clone();

        with_lease_paid::execute(self.lease.lease, Close::new(lease_account))
            .and_then(|close_msgs| {
                self.lease
                    .finalizer
                    .notify(customer)
                    .map(|finalizer_msgs| close_msgs.merge(finalizer_msgs)) //make sure the finalizer messages go out last
            })
            .map(|all_messages| MessageResponse::messages_with_events(all_messages, emitter))
            .map(|response| StateMachineResponse::from(response, Closed::default()))
    }
}

impl ContractInSwap for TransferIn {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.state(in_progress.into())
    }
}

impl From<Stage> for ClosingTrx {
    fn from(value: Stage) -> Self {
        match value {
            Stage::TransferOut => {
                // it's due to reusing the same enum dex::State
                // have to define a tailored enum dex::State that starts from TransferIn
                unreachable!(
                    "The lease asset transfer-in task never goes through a 'TransferOut' state!"
                )
            }
            Stage::Swap => {
                // it's due to reusing the same enum dex::State
                // have to define a tailored enum dex::State that starts from TransferIn
                unreachable!("The lease asset transfer-in task never goes through a 'Swap'!")
            }
            Stage::TransferInInit => Self::TransferInInit,
            Stage::TransferInFinish => Self::TransferInFinish,
        }
    }
}
