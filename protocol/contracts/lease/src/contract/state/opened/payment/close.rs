use serde::{Deserialize, Serialize};

use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::{
        query::opened::{OngoingTrx, PositionCloseTrx},
        LeaseCoin, LpnCoin,
    },
    contract::{
        cmd::{FullClose as FullCloseCmd, RepayEmitter},
        state::{opened::close::Closable, Response, State},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::Repayable;

pub(crate) trait CloseAlgo {
    type OutState: Default + Into<State>;

    type ProfitSender: FixedAddressSender;

    type ChangeSender: FixedAddressSender;

    type PaymentEmitter<'this, 'env>: RepayEmitter
    where
        Self: 'this,
        'env: 'this;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender;
    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender;
    fn emitter_fn<'this, 'lease, 'env>(
        &'this self,
        lease: &'lease Lease,
        env: &'env Env,
    ) -> Self::PaymentEmitter<'this, 'env>
    where
        Self: 'this,
        'env: 'this,
        'this: 'lease;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Close<CloseAlgoT>(CloseAlgoT)
where
    CloseAlgoT: CloseAlgo + Closable;

impl<CloseAlgoT> From<CloseAlgoT> for Close<CloseAlgoT>
where
    CloseAlgoT: CloseAlgo + Closable,
{
    fn from(value: CloseAlgoT) -> Self {
        Self(value)
    }
}

impl<CloseAlgoT> Closable for Close<CloseAlgoT>
where
    CloseAlgoT: CloseAlgo + Closable,
{
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin {
        self.0.amount(lease)
    }

    fn transaction(&self, lease: &Lease, in_progress: PositionCloseTrx) -> OngoingTrx {
        self.0.transaction(lease, in_progress)
    }

    fn event_type(&self) -> Type {
        self.0.event_type()
    }
}

impl<CloseAlgoT> Repayable for Close<CloseAlgoT>
where
    CloseAlgoT: CloseAlgo + Closable,
{
    fn try_repay(
        &self,
        lease: Lease,
        amount: LpnCoin,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        let customer = lease.lease.customer.clone();

        lease.finalizer.notify(customer).and_then(|finalizer_msgs| {
            let profit = self.0.profit_sender(&lease);
            let change = self.0.change_sender(&lease);
            let emitter_fn = self.0.emitter_fn(&lease, env);
            lease
                .lease
                .execute(
                    FullCloseCmd::new(amount, &env.block.time, profit, change, emitter_fn),
                    querier,
                )
                .map(|liquidation_response| liquidation_response.merge_with(finalizer_msgs))
                //make sure the finalizer messages go out last
                .map(|response| Response::from(response, CloseAlgoT::OutState::default()))
        })
    }
}
