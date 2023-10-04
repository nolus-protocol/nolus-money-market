use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        opened::{OngoingTrx, PositionCloseTrx},
        LeaseCoin,
    },
    contract::{
        cmd::{PartialCloseFn, PartialLiquidationDTO},
        state::{
            event::LiquidationEmitter,
            opened::{
                close::{self, Closable, IntoRepayable},
                payment::{Repay, RepayAlgo},
            },
        },
        Lease,
    },
    event::Type,
};

type Spec = PartialLiquidationDTO;
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
        OngoingTrx::Liquidation {
            liquidation: self.amount(lease).clone(),
            in_progress,
        }
    }

    fn event_type(&self) -> Type {
        Type::LiquidationSwap
    }
}

impl RepayAlgo for Spec {
    type RepayFn = PartialCloseFn;

    type PaymentEmitter<'liq, 'env> = LiquidationEmitter<'liq, 'env>;

    fn repay_fn(&self) -> Self::RepayFn {
        Self::RepayFn::new(self.amount.clone())
    }

    fn emitter_fn<'liq, 'env>(&'liq self, env: &'env Env) -> Self::PaymentEmitter<'liq, 'env> {
        Self::PaymentEmitter::new(&self.cause, self.amount.clone(), env)
    }
}
