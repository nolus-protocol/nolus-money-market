use serde::{Deserialize, Serialize};

use platform::bank::LazySenderStub;
use sdk::cosmwasm_std::Env;

use crate::{
    api::LeaseCoin,
    contract::{
        state::{
            closed::Closed,
            event::PositionCloseEmitter,
            opened::{
                close::{self, Closable},
                payment::{Close, CloseAlgo},
            },
        },
        Lease,
    },
    event::Type,
};

type Spec = FullClose;
pub(super) type RepayableImpl = Close<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

#[derive(Serialize, Deserialize)]
pub(crate) struct FullClose();

impl Closable for Spec {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin {
        &lease.lease.position.amount
    }

    fn event_type(&self) -> Type {
        Type::ClosePosition
    }
}

impl CloseAlgo for Spec {
    type OutState = Closed;

    type ProfitSender = LazySenderStub; //TODO deduce it somehow from ProfitRef?

    type ChangeSender = Self::ProfitSender;

    type PaymentEmitter<'this, 'env> = PositionCloseEmitter<'this, 'env>
    where
        Self: 'this,
        'env: 'this;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender {
        lease.lease.loan.profit().clone().into_stub()
    }

    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender {
        Self::ChangeSender::new(lease.lease.customer.clone())
    }

    fn emitter_fn<'this, 'env>(
        &'this self,
        lease: &'this Lease,
        env: &'env Env,
    ) -> Self::PaymentEmitter<'this, 'env>
    where
        'env: 'this,
    {
        Self::PaymentEmitter::new(self.amount(lease), env)
    }
}
