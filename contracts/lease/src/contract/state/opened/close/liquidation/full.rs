use sdk::cosmwasm_std::Env;

use platform::bank::LazySenderStub;

use crate::{
    api::LeaseCoin,
    contract::{
        cmd::FullLiquidationDTO,
        state::{
            event::LiquidationEmitter,
            liquidated::Liquidated,
            opened::{
                close::{self, Closable},
                payment::{Close, CloseAlgo},
            },
        },
        Lease,
    },
    event::Type,
};

type Spec = FullLiquidationDTO;
pub(super) type RepayableImpl = Close<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

impl Closable for Spec {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin {
        &lease.lease.position.amount
    }

    fn event_type(&self) -> Type {
        Type::LiquidationSwap
    }
}

impl CloseAlgo for Spec {
    type OutState = Liquidated;

    type ProfitSender = LazySenderStub; //TODO deduce it somehow from ProfitRef?

    type ChangeSender = Self::ProfitSender;

    type PaymentEmitter<'this, 'env> = LiquidationEmitter<'this, 'env>
    where
    Self: 'this,
    'env: 'this;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender {
        lease.lease.loan.profit().clone().into_stub()
    }

    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender {
        self.profit_sender(lease)
    }

    fn emitter_fn<'this, 'env>(
        &'this self,
        lease: &'this Lease,
        env: &'env Env,
    ) -> Self::PaymentEmitter<'this, 'env>
    where
        'env: 'this,
    {
        Self::PaymentEmitter::new(&self.cause, self.amount(lease), env)
    }
}