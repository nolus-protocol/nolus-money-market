use sdk::cosmwasm_std::Env;

use platform::bank::LazySenderStub;

use crate::contract::{
    cmd::{Closable, FullLiquidationDTO},
    state::{
        event::LiquidationEmitter,
        liquidated::Liquidated,
        opened::payment::{Close, CloseAlgo},
    },
    Lease,
};

type Spec = FullLiquidationDTO;
type RepayableImpl = Close<Spec>;
type Task = super::sell_asset::Task<RepayableImpl>;
pub(super) type StartState = super::sell_asset::StartState<RepayableImpl>;
pub(crate) type DexState = super::sell_asset::DexState<RepayableImpl>;

pub(in crate::contract::state) fn start(lease: Lease, spec: Spec) -> StartState {
    dex::start_remote_local(Task::new(lease, RepayableImpl::from(spec)))
}

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
