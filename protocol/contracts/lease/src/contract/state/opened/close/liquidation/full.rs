use profit::stub::ProfitStub;
use sdk::cosmwasm_std::Env;

use crate::{
    api::{
        LeaseCoin,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::{
        Lease,
        cmd::FullLiquidationDTO,
        state::{
            event::LiquidationEmitter,
            liquidated::Liquidated,
            opened::{
                close::{self, Closable, IntoRepayable},
                payment::{Close, CloseAlgo},
            },
        },
    },
    event::Type,
};

type Spec = FullLiquidationDTO;
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
        OngoingTrx::Liquidation {
            liquidation: *self.amount(lease),
            in_progress,
        }
    }

    fn event_type(&self) -> Type {
        Type::LiquidationSwap
    }
}

impl CloseAlgo for Spec {
    type OutState = Liquidated;

    type ProfitSender = ProfitStub;

    type ChangeSender = Self::ProfitSender;

    type PaymentEmitter<'this, 'env>
        = LiquidationEmitter<'this, 'env>
    where
        Self: 'this,
        'env: 'this;

    fn profit_sender(&self, lease: &Lease) -> Self::ProfitSender {
        lease.lease.loan.profit().clone().into_stub()
    }

    fn change_sender(&self, lease: &Lease) -> Self::ChangeSender {
        self.profit_sender(lease)
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
        Self::PaymentEmitter::new(&self.cause, *self.amount(lease), env)
    }
}
