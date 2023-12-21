use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        query::opened::{OngoingTrx, PositionCloseTrx},
        LeaseCoin, PartialClose,
    },
    contract::{
        cmd::PartialCloseFn,
        state::{
            event::PositionCloseEmitter,
            opened::{
                close::{self, Closable, IntoRepayable},
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

impl IntoRepayable for Spec {
    type Repayable = RepayableImpl;

    fn into(self) -> Self::Repayable {
        Into::into(self)
    }
}

impl Closable for Spec {
    fn amount(&self, _lease: &Lease) -> &LeaseCoin {
        &self.amount
    }

    fn transaction(&self, lease: &Lease, in_progress: PositionCloseTrx) -> OngoingTrx {
        OngoingTrx::Close {
            close: self.amount(lease).clone(),
            in_progress,
        }
    }

    fn event_type(&self) -> Type {
        Type::ClosePosition
    }
}

impl RepayAlgo for Spec {
    type RepayFn = PartialCloseFn;

    type PaymentEmitter<'this, 'env> = PositionCloseEmitter<'env>;

    fn repay_fn(&self) -> Self::RepayFn {
        Self::RepayFn::new(self.amount.clone())
    }

    fn emitter_fn<'this, 'env>(&'this self, env: &'env Env) -> Self::PaymentEmitter<'this, 'env> {
        Self::PaymentEmitter::new(self.amount.clone(), env)
    }
}
