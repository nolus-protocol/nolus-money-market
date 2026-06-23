use std::str;

use serde::{Deserialize, Serialize};

use enum_dispatch::enum_dispatch;

use finance::duration::Duration;
use platform::{
    batch::Batch,
    ica::{ErrorResponse as ICAErrorResponse, HostAccount},
    message::Response as MessageResponse,
};
use remote_lease::{callback::RemoteLeaseCallback, response::RemoteLeaseId};
use sdk::{
    cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply, Storage},
    cw_storage_plus::Item,
};

use crate::{
    api::{
        open::NewLeaseContract,
        position::{ClosePolicyChange, PositionClose},
        query::StateResponse,
    },
    contract::api::Contract,
    error::ContractResult,
};

pub(crate) use self::handler::{Handler, Response};
use self::{dex::State as DexState, lease::State as LeaseState};
use finance::instant::Instant;

mod closed;
mod dex;
mod drain;
mod event;
mod handler;
mod lease;
mod liquidated;
mod open_failed;
mod opened;
mod opening;
mod out_task;
mod paid;
mod resp_delivery;

type RequestLoan = LeaseState<opening::request_loan::RequestLoan>;

type OpenLease = opening::open_lease::OpenLease;

type OpenFailed = open_failed::OpenFailed;

type BuyAsset = DexState<opening::buy_asset::DexState>;

type OpeningUnwind = DexState<opening::buy_asset::UnwindState>;

type OpenedActive = LeaseState<opened::active::Active>;

type BuyLpn = DexState<opened::repay::buy_lpn::DexState>;

type BuyLpnDrain = DexState<opened::repay::buy_lpn::DrainState>;

type PartialLiquidation = DexState<opened::close::sell_asset::liquidation::partial::DexState>;

type PartialLiquidationDrain = DexState<opened::close::sell_asset::liquidation::PartialDrainState>;

type FullLiquidation = DexState<opened::close::sell_asset::liquidation::full::DexState>;

type FullLiquidationDrain = DexState<opened::close::sell_asset::liquidation::FullDrainState>;

type PartialClose = DexState<opened::close::sell_asset::customer_close::partial::DexState>;

type PartialCloseDrain = DexState<opened::close::sell_asset::customer_close::PartialDrainState>;

type FullClose = DexState<opened::close::sell_asset::customer_close::full::DexState>;

type FullCloseDrain = DexState<opened::close::sell_asset::customer_close::FullDrainState>;

type ClosingTransferOut = DexState<paid::transfer_out::DexState>;

type ClosingRemoteLease = LeaseState<paid::remote_close::ClosingRemoteLease>;

type Closed = LeaseState<closed::Closed>;

type Liquidated = LeaseState<liquidated::Liquidated>;

type SwapResult = ContractResult<Response>;

/// Bridge a Solana-side `LeaseAuthority` from the wire `RemoteLeaseId` into the
/// `HostAccount` the funding ICS-20 transfers address. The base58
/// `RemoteLeaseId` is always non-empty, so the host-account validation never
/// rejects it. Shared by the opening funding leg (downpayment + principal) and
/// the repay funding leg (the payment).
pub(crate) fn remote_lease_host(remote_lease_id: &RemoteLeaseId) -> ContractResult<HostAccount> {
    HostAccount::try_from(remote_lease_id.as_str().to_owned()).map_err(Into::into)
}

#[enum_dispatch(Contract)]
#[derive(Serialize, Deserialize)]
pub enum State {
    RequestLoan,
    OpenLease,
    OpenFailed,
    BuyAsset,
    OpeningUnwind,
    OpenedActive,
    BuyLpn,
    BuyLpnDrain,
    PartialLiquidation,
    PartialLiquidationDrain,
    FullLiquidation,
    FullLiquidationDrain,
    PartialClose,
    PartialCloseDrain,
    FullClose,
    FullCloseDrain,
    ClosingTransferOut,
    ClosingRemoteLease,
    Closed,
    Liquidated,
}

const STATE_DB_ITEM: Item<State> = Item::new("state");

pub(super) fn load(storage: &dyn Storage) -> ContractResult<State> {
    STATE_DB_ITEM.load(storage).map_err(Into::into)
}

pub(super) fn save(storage: &mut dyn Storage, next_state: &State) -> ContractResult<()> {
    STATE_DB_ITEM.save(storage, next_state).map_err(Into::into)
}

pub(super) fn new_lease(
    querier: QuerierWrapper<'_>,
    info: MessageInfo,
    spec: NewLeaseContract,
) -> ContractResult<(Batch, State)> {
    opening::request_loan::RequestLoan::new(querier, info, spec)
        .map(|(batch, start_state)| (batch, start_state.into()))
}

fn ignore_msg<S>(state: S) -> ContractResult<Response>
where
    S: Into<State>,
{
    Ok(Response::from(MessageResponse::default(), state))
}

mod impl_from {
    use super::{
        BuyAsset, BuyLpn, BuyLpnDrain, Closed, ClosingRemoteLease, ClosingTransferOut, FullClose,
        FullCloseDrain, FullLiquidation, FullLiquidationDrain, Liquidated, OpenedActive,
        OpeningUnwind, PartialClose, PartialCloseDrain, PartialLiquidation,
        PartialLiquidationDrain, RequestLoan, State,
    };

    impl From<super::opening::request_loan::RequestLoan> for State {
        fn from(value: super::opening::request_loan::RequestLoan) -> Self {
            RequestLoan::new(value).into()
        }
    }

    impl From<super::opening::buy_asset::DexState> for State {
        fn from(value: super::opening::buy_asset::DexState) -> Self {
            BuyAsset::new(value).into()
        }
    }

    impl From<super::opening::buy_asset::UnwindState> for State {
        fn from(value: super::opening::buy_asset::UnwindState) -> Self {
            OpeningUnwind::new(value).into()
        }
    }

    impl From<super::opened::active::Active> for State {
        fn from(value: super::opened::active::Active) -> Self {
            OpenedActive::new(value).into()
        }
    }

    impl From<super::opened::repay::buy_lpn::DexState> for State {
        fn from(value: super::opened::repay::buy_lpn::DexState) -> Self {
            BuyLpn::new(value).into()
        }
    }

    impl From<super::opened::repay::buy_lpn::DrainState> for State {
        fn from(value: super::opened::repay::buy_lpn::DrainState) -> Self {
            BuyLpnDrain::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::liquidation::partial::DexState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::partial::DexState) -> Self {
            PartialLiquidation::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::liquidation::PartialDrainState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::PartialDrainState) -> Self {
            PartialLiquidationDrain::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::liquidation::full::DexState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::full::DexState) -> Self {
            FullLiquidation::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::liquidation::FullDrainState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::FullDrainState) -> Self {
            FullLiquidationDrain::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::partial::DexState> for State {
        fn from(
            value: super::opened::close::sell_asset::customer_close::partial::DexState,
        ) -> Self {
            PartialClose::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::PartialDrainState> for State {
        fn from(
            value: super::opened::close::sell_asset::customer_close::PartialDrainState,
        ) -> Self {
            PartialCloseDrain::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::full::DexState> for State {
        fn from(value: super::opened::close::sell_asset::customer_close::full::DexState) -> Self {
            FullClose::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::FullDrainState> for State {
        fn from(value: super::opened::close::sell_asset::customer_close::FullDrainState) -> Self {
            FullCloseDrain::new(value).into()
        }
    }

    impl From<super::paid::transfer_out::DexState> for State {
        fn from(value: super::paid::transfer_out::DexState) -> Self {
            ClosingTransferOut::new(value).into()
        }
    }

    impl From<super::paid::remote_close::ClosingRemoteLease> for State {
        fn from(value: super::paid::remote_close::ClosingRemoteLease) -> Self {
            ClosingRemoteLease::new(value).into()
        }
    }

    impl From<super::closed::Closed> for State {
        fn from(value: super::closed::Closed) -> Self {
            Closed::new(value).into()
        }
    }

    impl From<super::liquidated::Liquidated> for State {
        fn from(value: super::liquidated::Liquidated) -> Self {
            Liquidated::new(value).into()
        }
    }
}
