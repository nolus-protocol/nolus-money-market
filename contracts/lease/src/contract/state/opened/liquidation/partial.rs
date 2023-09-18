use sdk::cosmwasm_std::Env;

use crate::contract::{
    cmd::{PartialCloseFn, PartialLiquidationDTO},
    state::{
        event::LiquidationEmitter,
        opened::payment::{Repay, RepayAlgo},
    },
    Lease,
};

type Spec = PartialLiquidationDTO;
type RepayableImpl = Repay<Spec>;
type Task = super::sell_asset::Task<RepayableImpl>;
pub(super) type StartState = super::sell_asset::StartState<RepayableImpl>;
pub(crate) type DexState = super::sell_asset::DexState<RepayableImpl>;

pub(in crate::contract::state) fn start(lease: Lease, spec: Spec) -> StartState {
    dex::start_remote_local(Task::new(lease, RepayableImpl::from(spec)))
}

impl RepayAlgo for Spec {
    type RepayFn = PartialCloseFn;

    type PaymentEmitter<'liq, 'env> = LiquidationEmitter<'liq, 'env>;

    fn repay_fn(&self) -> Self::RepayFn {
        Self::RepayFn::new(self.amount.clone())
    }

    fn emitter_fn<'liq, 'env>(&'liq self, env: &'env Env) -> Self::PaymentEmitter<'liq, 'env> {
        Self::PaymentEmitter::new(&self.cause, &self.amount, env)
    }
}
