use sdk::cosmwasm_std::Env;

use crate::contract::{
    cmd::{PartialCloseFn, PartialLiquidationDTO},
    state::{
        event::LiquidationEmitter,
        opened::{
            close,
            payment::{Repay, RepayAlgo},
        },
    },
};

type Spec = PartialLiquidationDTO;
pub(super) type RepayableImpl = Repay<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

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
