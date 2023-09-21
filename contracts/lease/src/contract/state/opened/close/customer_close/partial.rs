use sdk::cosmwasm_std::Env;

use crate::{
    api::{LeaseCoin, PartialClose},
    contract::{
        cmd::PartialCloseFn,
        state::{
            event::PositionCloseEmitter,
            opened::{
                close::{self, Closable},
                payment::{Repay, RepayAlgo},
            },
        },
        Lease,
    },
    event::Type,
};

type Spec = PartialClose;
pub(super) type RepayableImpl = Repay<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

impl Closable for Spec {
    fn amount(&self, _lease: &Lease) -> &LeaseCoin {
        &self.amount
    }

    fn event_type(&self) -> Type {
        Type::ClosePosition
    }
}

impl RepayAlgo for Spec {
    type RepayFn = PartialCloseFn;

    type PaymentEmitter<'close, 'env> = PositionCloseEmitter<'close, 'env>;

    fn repay_fn(&self) -> Self::RepayFn {
        Self::RepayFn::new(self.amount.clone())
    }

    fn emitter_fn<'this, 'env>(&'this self, env: &'env Env) -> Self::PaymentEmitter<'this, 'env> {
        Self::PaymentEmitter::new(&self.amount, env)
    }
}
