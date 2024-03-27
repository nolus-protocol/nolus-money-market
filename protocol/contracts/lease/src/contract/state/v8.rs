use serde::{Deserialize, Serialize, Serializer};

use dex::MigrateSpec;

use crate::finance::ReserveRef;

use super::{
    closed,
    dex::State as DexState,
    lease::State as LeaseState,
    liquidated,
    opened::{self},
    opening, paid, Response,
};

type BuyAsset = DexState<opening::v8::DexState>;

type OpenedActive = LeaseState<opened::v8::Active>;

type BuyLpn = DexState<opened::repay::v8::DexState>;

type PartialLiquidation = DexState<opened::close::v8::PartialLiquidationDexState>;

type FullLiquidation = DexState<opened::close::v8::FullLiquidationDexState>;

type PartialClose = DexState<opened::close::v8::PartialCloseDexState>;

type FullClose = DexState<opened::close::v8::FullCloseDexState>;

type PaidActive = LeaseState<paid::v8::Active>;

type ClosingTransferIn = DexState<paid::v8::DexState>;

type Closed = LeaseState<closed::Closed>;

type Liquidated = LeaseState<liquidated::Liquidated>;

pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self, reserve: ReserveRef) -> Response;
}

#[derive(Deserialize)]
pub(in crate::contract) enum State {
    // RequestLoan(RequestLoan), not a persistent state so we skip it
    BuyAsset(BuyAsset),
    OpenedActive(OpenedActive),
    BuyLpn(BuyLpn),
    PartialLiquidation(PartialLiquidation),
    FullLiquidation(FullLiquidation),
    PartialClose(PartialClose),
    FullClose(FullClose),
    PaidActive(PaidActive),
    ClosingTransferIn(ClosingTransferIn),
    Closed(Closed),
    Liquidated(Liquidated),
}

impl Migrate for State {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        match self {
            Self::BuyAsset(inner) => inner.into_last_version(reserve),
            Self::OpenedActive(inner) => inner.into_last_version(reserve),
            Self::BuyLpn(inner) => inner.into_last_version(reserve),
            Self::PartialLiquidation(inner) => inner.into_last_version(reserve),
            Self::FullLiquidation(inner) => inner.into_last_version(reserve),
            Self::PartialClose(inner) => inner.into_last_version(reserve),
            Self::FullClose(inner) => inner.into_last_version(reserve),
            Self::PaidActive(inner) => inner.into_last_version(reserve),
            Self::ClosingTransferIn(inner) => inner.into_last_version(reserve),
            Self::Closed(inner) => inner.into_last_version(reserve),
            Self::Liquidated(inner) => inner.into_last_version(reserve),
        }
    }
}

impl Migrate for BuyAsset {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            let reserve_clone = reserve.clone();
            dex.migrate(
                |open_ica| open_ica.migrate(reserve_clone),
                |spec| spec.migrate(reserve),
            )
        }))
    }
}

impl Migrate for OpenedActive {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|active| active.migrate(reserve)))
    }
}

impl Migrate for BuyLpn {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::repay::v8::BuyLpn,
                opened::repay::buy_lpn::BuyLpn,
                opened::repay::buy_lpn::DexState,
            >::migrate_spec(dex, |spec| spec.migrate(reserve))
        }))
    }
}

impl Migrate for PartialLiquidation {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::close::v8::PartialLiquidationTask,
                opened::close::liquidation::partial::Task,
                opened::close::liquidation::partial::DexState,
            >::migrate_spec(dex, |task| task.migrate(reserve))
        }))
    }
}

impl Migrate for FullLiquidation {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::close::v8::FullLiquidationDexTask,
                opened::close::liquidation::full::Task,
                opened::close::liquidation::full::DexState,
            >::migrate_spec(dex, |task| task.migrate(reserve))
        }))
    }
}

impl Migrate for PartialClose {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::close::v8::PartialCloseTask,
                opened::close::customer_close::partial::Task,
                opened::close::customer_close::partial::DexState,
            >::migrate_spec(dex, |task| task.migrate(reserve))
        }))
    }
}

impl Migrate for FullClose {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::close::v8::FullCloseTask,
                opened::close::customer_close::full::Task,
                opened::close::customer_close::full::DexState,
            >::migrate_spec(dex, |task| task.migrate(reserve))
        }))
    }
}

impl Migrate for PaidActive {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|active| active.migrate(reserve)))
    }
}
impl Migrate for ClosingTransferIn {
    fn into_last_version(self, reserve: ReserveRef) -> Response {
        Response::no_msgs(self.map(|dex| {
            MigrateSpec::<_, _, paid::transfer_in::DexState>::migrate_spec(dex, |spec| {
                spec.migrate(reserve)
            })
        }))
    }
}
impl Migrate for Closed {
    fn into_last_version(self, _reserve: ReserveRef) -> Response {
        Response::no_msgs(self)
    }
}
impl Migrate for Liquidated {
    fn into_last_version(self, _reserve: ReserveRef) -> Response {
        Response::no_msgs(self)
    }
}

impl Serialize for State {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unimplemented!("required by a cosmwasm_std::Iten::load trait bound")
    }
}
