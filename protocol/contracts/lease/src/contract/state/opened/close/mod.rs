use dex::{AnomalyHandler, SlippageCalculator};

use crate::{
    api::{
        LeaseAssetCurrencies, LeaseCoin,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::Lease,
    event::Type,
    finance::LpnCurrency,
};

pub(crate) use anomaly::SlippageAnomaly;

use super::payment::Repayable;

mod anomaly;
pub mod sell_asset;

pub(crate) trait Closable {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin;
    fn transaction(&self, lease: &Lease, in_progress: PositionCloseTrx) -> OngoingTrx;
    fn event_type(&self) -> Type;
}

/// Aim to simplify trait boundaries within this module and underneat
pub(crate) trait Calculator
where
    Self: SlippageCalculator<LeaseAssetCurrencies, OutC = LpnCurrency>,
{
}

trait IntoRepayable
where
    Self::Repayable: Closable + Repayable,
{
    type Repayable;

    fn into(self) -> Self::Repayable;
}
