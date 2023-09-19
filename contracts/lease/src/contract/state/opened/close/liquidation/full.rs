use sdk::cosmwasm_std::Env;

use platform::bank::LazySenderStub;

use crate::contract::{
    cmd::{Closable, FullLiquidationDTO},
    state::{
        event::LiquidationEmitter,
        liquidated::Liquidated,
        opened::{
            close,
            payment::{Close, CloseAlgo},
        },
    },
    Lease,
};

type Spec = FullLiquidationDTO;
pub(super) type RepayableImpl = Close<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

impl CloseAlgo for Spec {
    type OutState = Liquidated;

    type ProfitSender = LazySenderStub; //TODO deduce it somehow from ProfitRef?

    type ChangeSender = Self::ProfitSender;

    type PaymentEmitter<'liq, 'env> = LiquidationEmitter<'liq, 'env>;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender {
        lease.lease.loan.profit().clone().into_stub()
    }

    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender {
        self.profit_sender(lease)
    }

    fn emitter_fn<'liq, 'env>(
        &'liq self,
        lease: &'liq Lease,
        env: &'env Env,
    ) -> Self::PaymentEmitter<'liq, 'env> {
        Self::PaymentEmitter::new(&self.cause, self.amount(lease), env)
    }
}
