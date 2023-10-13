use serde::{Deserialize, Serialize, Serializer};

use dex::{InspectSpec, MigrateSpec};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{contract::finalize::FinalizerRef, error::ContractResult};

use super::{
    closed,
    dex::State as DexState,
    lease::State as LeaseState,
    liquidated, opened,
    opening::{self},
    paid, Closed, Liquidated, Response, State as State_v6,
};

type BuyAsset = DexState<opening::v5::DexState>;

type OpenedActive = LeaseState<opened::v5::Active>;

type BuyLpn = DexState<opened::repay::v5::DexState>;

type SellAsset = DexState<opened::close::liquidation::v5::DexState>;

type PaidActive = LeaseState<paid::v5::Active>;

type ClosingTransferIn = DexState<paid::v5::DexState>;

pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(
        self,
        now: Timestamp,
        customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response>;
}

#[derive(Deserialize)]
pub(in crate::contract) enum State {
    // RequestLoan, not a persistent state so we skip it
    BuyAsset(BuyAsset),
    OpenedActive(OpenedActive),
    BuyLpn(BuyLpn),
    SellAsset(SellAsset),
    PaidActive(PaidActive),
    ClosingTransferIn(ClosingTransferIn),
    Closed(Closed),
    Liquidated(Liquidated),
}

impl Migrate for State {
    fn into_last_version(
        self,
        now: Timestamp,
        customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        match self {
            Self::BuyAsset(inner) => inner.into_last_version(now, customer, finalizer),
            Self::OpenedActive(inner) => inner.into_last_version(now, customer, finalizer),
            Self::BuyLpn(inner) => inner.into_last_version(now, customer, finalizer),
            Self::SellAsset(inner) => inner.into_last_version(now, customer, finalizer),
            Self::PaidActive(inner) => inner.into_last_version(now, customer, finalizer),
            Self::ClosingTransferIn(inner) => inner.into_last_version(now, customer, finalizer),
            Self::Closed(inner) => inner.into_last_version(now, customer, finalizer),
            Self::Liquidated(inner) => inner.into_last_version(now, customer, finalizer),
        }
    }
}

impl Migrate for BuyAsset {
    fn into_last_version(
        self,
        now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        Ok(Response::no_msgs(self.map(|dex| {
            let f_clone = finalizer.clone();
            dex.migrate(
                |open_ica| open_ica.migrate(f_clone, now),
                |spec| spec.migrate(finalizer, now),
            )
        })))
    }
}

impl Migrate for OpenedActive {
    fn into_last_version(
        self,
        _now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        Ok(Response::no_msgs(
            self.map(|active| active.migrate(finalizer)),
        ))
    }
}

impl Migrate for BuyLpn {
    fn into_last_version(
        self,
        _now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        Ok(Response::no_msgs(self.map(|dex| {
            MigrateSpec::<
                opened::repay::v5::BuyLpn,
                opened::repay::buy_lpn::BuyLpn,
                opened::repay::buy_lpn::DexState,
            >::migrate_spec(dex, |spec| spec.migrate(finalizer))
        })))
    }
}

impl Migrate for SellAsset {
    fn into_last_version(
        self,
        _now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        let partial: bool = self.inspect(|dex| dex.inspect_spec(|spec| spec.partial()));

        let next_state: State_v6 = if partial {
            self.map(|dex| {
                MigrateSpec::<_, _, opened::close::liquidation::partial::DexState>::migrate_spec(
                    dex,
                    |spec| spec.migrate_into_partial(finalizer),
                )
            })
            .into()
        } else {
            self.map(|dex| {
                MigrateSpec::<_, _, opened::close::liquidation::full::DexState>::migrate_spec(
                    dex,
                    |spec| spec.migrate_into_full(finalizer),
                )
            })
            .into()
        };
        Ok(Response::no_msgs(next_state))
    }
}

impl Migrate for PaidActive {
    fn into_last_version(
        self,
        _now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        Ok(Response::no_msgs(
            self.map(|active| active.migrate(finalizer)),
        ))
    }
}
impl Migrate for ClosingTransferIn {
    fn into_last_version(
        self,
        _now: Timestamp,
        _customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        Ok(Response::no_msgs(self.map(|dex| {
            MigrateSpec::<_, _, paid::transfer_in::DexState>::migrate_spec(dex, |spec| {
                spec.migrate(finalizer)
            })
        })))
    }
}
impl Migrate for Closed {
    fn into_last_version(
        self,
        _now: Timestamp,
        customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        finalizer.notify(customer).map(|finalize_msg| {
            Response::from(
                MessageResponse::messages_only(finalize_msg),
                self.map::<_, closed::Closed>(Into::into),
            )
        })
    }
}
impl Migrate for Liquidated {
    fn into_last_version(
        self,
        _now: Timestamp,
        customer: Addr,
        finalizer: FinalizerRef,
    ) -> ContractResult<Response> {
        finalizer.notify(customer).map(|finalize_msg| {
            Response::from(
                MessageResponse::messages_only(finalize_msg),
                self.map::<_, liquidated::Liquidated>(Into::into),
            )
        })
    }
}

impl Serialize for State {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
