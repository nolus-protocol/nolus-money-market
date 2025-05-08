use dex::{
    AcceptAnyNonZeroSwap, AnomalyTreatment, SlippageCalculator, SlippageCalculatorFactory,
    SwapOutputTask,
};
use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        LeaseCoin,
        position::PartialClose,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::{
        Lease,
        cmd::PartialCloseFn,
        state::{
            event::PositionCloseEmitter,
            opened::{
                close::{self, AnomalyHandler, Closable, IntoRepayable, sell_asset::SellAsset},
                payment::{Repay, RepayAlgo},
            },
        },
    },
    event::Type,
    finance::LpnCurrency,
};

type Spec = PartialClose;
pub(in super::super) type RepayableImpl = Repay<Spec>;
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
            close: *self.amount(lease),
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
        Self::RepayFn::new(self.amount)
    }

    fn emitter_fn<'this, 'env>(&'this self, env: &'env Env) -> Self::PaymentEmitter<'this, 'env> {
        Self::PaymentEmitter::new(self.amount, env)
    }
}

impl SlippageCalculatorFactory<SellAsset<RepayableImpl>> for SellAsset<RepayableImpl> {
    type OutC = LpnCurrency;

    fn new_calc(&self) -> impl SlippageCalculator<SellAsset<RepayableImpl>, OutC = Self::OutC> {
        AcceptAnyNonZeroSwap::<
            _,
            <SellAsset<RepayableImpl> as SwapOutputTask<SellAsset<RepayableImpl>>>::OutC,
        >::default()
    }
}

impl AnomalyHandler<SellAsset<RepayableImpl>> for SellAsset<RepayableImpl> {
    fn on_anomaly(self) -> AnomalyTreatment<SellAsset<RepayableImpl>> {
        self.retry_on_anomaly()
    }
}
