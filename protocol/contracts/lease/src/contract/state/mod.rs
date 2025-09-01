use std::str;

use enum_dispatch::enum_dispatch;
use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use platform::{
    batch::Batch, ica::ErrorResponse as ICAErrorResponse, message::Response as MessageResponse,
};
use sdk::{
    cosmwasm_std::{
        Binary, Env, MessageInfo, QuerierWrapper, Reply, StdError as CwError, Storage, Timestamp,
    },
    cw_storage_plus::Item,
};
use swap::Impl;

use crate::{
    api::{
        open::NewLeaseContract,
        position::{ClosePolicyChange, PositionClose},
        query::StateResponse,
    },
    contract::api::Contract,
    error::{ContractError, ContractResult},
};

pub(crate) use self::handler::{Handler, Response};
use self::{dex::State as DexState, lease::State as LeaseState};

mod closed;
mod dex;
mod drain;
mod event;
mod handler;
mod lease;
mod liquidated;
mod opened;
mod opening;
mod out_task;
mod paid;
mod resp_delivery;

type RequestLoan = LeaseState<opening::request_loan::RequestLoan>;

type BuyAsset = DexState<opening::buy_asset::DexState>;

type OpenedActive = LeaseState<opened::active::Active>;

type BuyLpn = DexState<opened::repay::buy_lpn::DexState>;

type PartialLiquidation = DexState<opened::close::sell_asset::liquidation::partial::DexState>;

type FullLiquidation = DexState<opened::close::sell_asset::liquidation::full::DexState>;

type SlippageAnomaly = LeaseState<opened::close::SlippageAnomaly>;

type PartialClose = DexState<opened::close::sell_asset::customer_close::partial::DexState>;

type FullClose = DexState<opened::close::sell_asset::customer_close::full::DexState>;

type ClosingTransferIn = DexState<paid::transfer_in::DexState>;

type Closed = LeaseState<closed::Closed>;

type Liquidated = LeaseState<liquidated::Liquidated>;

type SwapResult = ContractResult<Response>;

type SwapClient = Impl;

#[enum_dispatch(Contract)]
#[derive(Serialize, Deserialize)]
pub enum State {
    RequestLoan,
    BuyAsset,
    OpenedActive,
    BuyLpn,
    PartialLiquidation,
    FullLiquidation,
    SlippageAnomaly,
    PartialClose,
    FullClose,
    ClosingTransferIn,
    Closed,
    Liquidated,
}

const STATE_DB_ITEM: Item<State> = Item::new("state");

pub(super) fn load(storage: &dyn Storage) -> ContractResult<State> {
    STATE_DB_ITEM
        .load(storage)
        .map_err(|error: CwError| ContractError::Std(error.to_string()))
}

pub(super) fn save(storage: &mut dyn Storage, next_state: &State) -> ContractResult<()> {
    STATE_DB_ITEM
        .save(storage, next_state)
        .map_err(|error: CwError| ContractError::Std(error.to_string()))
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
        BuyAsset, BuyLpn, Closed, ClosingTransferIn, FullClose, FullLiquidation, Liquidated,
        OpenedActive, PartialClose, PartialLiquidation, RequestLoan, SlippageAnomaly, State,
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

    impl From<super::opened::close::sell_asset::liquidation::partial::DexState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::partial::DexState) -> Self {
            PartialLiquidation::new(value).into()
        }
    }

    impl From<super::opened::close::SlippageAnomaly> for State {
        fn from(value: super::opened::close::SlippageAnomaly) -> Self {
            SlippageAnomaly::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::liquidation::full::DexState> for State {
        fn from(value: super::opened::close::sell_asset::liquidation::full::DexState) -> Self {
            FullLiquidation::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::partial::DexState> for State {
        fn from(
            value: super::opened::close::sell_asset::customer_close::partial::DexState,
        ) -> Self {
            PartialClose::new(value).into()
        }
    }

    impl From<super::opened::close::sell_asset::customer_close::full::DexState> for State {
        fn from(value: super::opened::close::sell_asset::customer_close::full::DexState) -> Self {
            FullClose::new(value).into()
        }
    }

    impl From<super::paid::transfer_in::DexState> for State {
        fn from(value: super::paid::transfer_in::DexState) -> Self {
            ClosingTransferIn::new(value).into()
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
