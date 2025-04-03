use dex::{AcceptAnyNonZeroSwap, AnomalyMonitoredTask, AnomalyPolicy};
use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        LeaseCoin,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::{
        Lease,
        cmd::{PartialCloseFn, PartialLiquidationDTO},
        state::{
            SlippageAnomaly,
            event::LiquidationEmitter,
            opened::{
                close::{self, Closable, IntoRepayable, sell_asset::SellAsset},
                payment::{Repay, RepayAlgo},
            },
        },
    },
    event::Type,
};

type Spec = PartialLiquidationDTO;
pub(in super::super) type RepayableImpl = Repay<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;
pub(crate) type AnomalyState = SlippageAnomaly<RepayableImpl>;

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
            liquidation: *self.amount(lease),
            cause: self.cause.into(),
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
        Self::RepayFn::new(self.amount)
    }

    fn emitter_fn<'liq, 'env>(&'liq self, env: &'env Env) -> Self::PaymentEmitter<'liq, 'env> {
        Self::PaymentEmitter::new(&self.cause, self.amount, env)
    }
}

impl AnomalyMonitoredTask for SellAsset<RepayableImpl> {
    fn policy(&self) -> impl AnomalyPolicy<Self> {
        // TODO switch to MaxSlippage::on_task(self) once the MaxSlippage TODO's get resolved
        AcceptAnyNonZeroSwap::on_task(self)
    }
}
