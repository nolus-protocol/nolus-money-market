use std::{iter, marker::PhantomData};

use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDef, Group, MemberOf, never};
use dex::{
    Account, AnomalyTreatment, ContractInSwap, Stage, StartTransferInState, SwapOutputTask,
    SwapTask, WithCalculator, WithOutputTask,
};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
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
        LeaseAssetCurrencies,
        query::{StateResponse as QueryStateResponse, paid::ClosingTrx},
    },
    contract::{
        Lease,
        cmd::Close,
        state::{
            SwapClient, SwapResult,
            closed::Closed,
            out_task::{OutTaskFactory, WithOutCurrency},
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::ContractResult,
    event::Type,
    lease::{LeaseDTO, with_lease_paid},
};

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState = StartTransferInState<TransferIn, SwapClient, ForwardToDexEntry>;
pub(in super::super) type DexState = dex::StateLocalOut<TransferIn, SwapClient, ForwardToDexEntry>;

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

    fn amount(&self) -> &CoinDTO<AssetGroup> {
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
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::ClosingTransferIn
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
        iter::once(*self.amount())
    }

    fn with_slippage_calc<Cmd>(&self, _cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithCalculator<Self>,
    {
        unimplemented!("TransferIn is not subject to monitoring! No swaps included!")
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        struct OutputTaskFactory {}
        impl OutTaskFactory<TransferIn> for OutputTaskFactory {
            fn new_task<OutC>(swap_task: TransferIn) -> impl SwapOutputTask<TransferIn, OutC = OutC>
            where
                OutC: CurrencyDef,
                OutC::Group: MemberOf<<TransferIn as SwapTask>::OutG>
                    + MemberOf<<<TransferIn as SwapTask>::InG as Group>::TopG>,
            {
                TransferInFinish::<_, OutC>::from(swap_task)
            }
        }
        never::safe_unwrap(
            self.amount()
                .currency()
                .into_currency_type(WithOutCurrency::<_, OutputTaskFactory, _>::from(self, cmd)),
        )
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

struct TransferInFinish<SwapTask, OutC> {
    swap_task: SwapTask,
    _out_c: PhantomData<OutC>,
}

impl<SwapTask, OutC> TransferInFinish<SwapTask, OutC> {
    fn from(swap_task: SwapTask) -> Self {
        Self {
            swap_task,
            _out_c: PhantomData,
        }
    }
}

impl<OutC> SwapOutputTask<TransferIn> for TransferInFinish<TransferIn, OutC>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<TransferIn as SwapTask>::OutG>
        + MemberOf<<<TransferIn as SwapTask>::InG as Group>::TopG>,
{
    type OutC = OutC;

    fn as_spec(&self) -> &TransferIn {
        &self.swap_task
    }

    fn into_spec(self) -> TransferIn {
        self.swap_task
    }

    fn on_anomaly(self) -> AnomalyTreatment<TransferIn>
    where
        Self: Sized,
    {
        AnomalyTreatment::Retry(self.into_spec())
    }

    fn finish(
        self,
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <TransferIn as SwapTask>::Result {
        debug_assert!(
            &CoinDTO::<<TransferIn as SwapTask>::OutG>::from(amount_out) == self.as_spec().amount()
        );

        let spec = self.into_spec();
        let lease_addr = spec.lease.lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        let emitter = spec.emit_ok(env, &spec.lease.lease);
        let customer = spec.lease.lease.customer.clone();

        with_lease_paid::execute(spec.lease.lease, Close::new(lease_account))
            .and_then(|close_msgs| {
                spec.lease
                    .leases
                    .finalize_lease(customer)
                    .map(|finalizer_msgs| close_msgs.merge(finalizer_msgs)) //make sure the finalizer messages go out last
            })
            .map(|all_messages| MessageResponse::messages_with_event(all_messages, emitter))
            .map(|response| StateMachineResponse::from(response, Closed::default()))
    }
}
