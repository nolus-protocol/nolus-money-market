use dex::{
    AcceptAnyNonZeroSwap, AnomalyTreatment, SlippageCalculator, SlippageCalculatorFactory,
    SwapOutputTask,
};
use profit::stub::ProfitStub;
use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        LeaseCoin,
        position::FullClose,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::{
        Lease,
        state::{
            closed::Closed,
            event::PositionCloseEmitter,
            opened::{
                close::{self, AnomalyHandler, Closable, IntoRepayable, sell_asset::SellAsset},
                payment::{Close, CloseAlgo},
            },
        },
    },
    event::Type,
    finance::LpnCurrency,
};

type Spec = FullClose;
pub(in super::super) type RepayableImpl = Close<Spec>;
pub(crate) type DexState = close::DexState<RepayableImpl>;

impl IntoRepayable for Spec {
    type Repayable = RepayableImpl;

    fn into(self) -> Self::Repayable {
        Into::into(self)
    }
}

impl Closable for Spec {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin {
        lease.lease.position.amount()
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

impl CloseAlgo for Spec {
    type OutState = Closed;

    type ProfitSender = ProfitStub;

    type ChangeSender = Self::ProfitSender;

    type PaymentEmitter<'this, 'env>
        = PositionCloseEmitter<'env>
    where
        Self: 'this,
        'env: 'this;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender {
        lease.lease.loan.profit().clone().into_stub()
    }

    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender {
        Self::ChangeSender::new(lease.lease.customer.clone())
    }

    fn emitter_fn<'this, 'lease, 'env>(
        &'this self,
        lease: &'lease Lease,
        env: &'env Env,
    ) -> Self::PaymentEmitter<'this, 'env>
    where
        'env: 'this,
        'this: 'lease,
    {
        Self::PaymentEmitter::new(*self.amount(lease), env)
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
