use std::str;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::{
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply, Storage},
    cw_storage_plus::Item,
};

use crate::{api::NewLeaseContract, error::ContractResult};

pub(crate) use self::handler::{Handler, Response};
#[cfg(feature = "migration")]
pub(in crate::contract) use self::v2::{Migrate, StateV2};
use self::{dex::State as DexState, lease::State as LeaseState};

mod closed;
mod dex;
mod handler;
mod lease;
mod liquidated;
mod opened;
mod opening;
mod paid;
#[cfg(feature = "migration")]
mod v2;

type RequestLoan = LeaseState<opening::request_loan::RequestLoan>;

type OpenIcaAccount = DexState<::dex::IcaConnector<opening::open_ica::OpenIcaAccount, SwapResult>>;

type BuyAsset = DexState<opening::buy_asset::DexState>;

type OpenedActive = LeaseState<opened::active::Active>;

type BuyLpn = DexState<opened::repay::buy_lpn::DexState>;

type SellAsset = DexState<opened::liquidation::sell_asset::DexState>;

type PaidActive = LeaseState<paid::Active>;

type ClosingTransferIn = DexState<paid::transfer_in::DexState>;

type Closed = LeaseState<closed::Closed>;

type Liquidated = LeaseState<liquidated::Liquidated>;

type SwapResult = ContractResult<Response>;

#[enum_dispatch(Handler, Contract)]
#[derive(Serialize, Deserialize)]
pub(crate) enum State {
    RequestLoan,
    OpenIcaAccount,
    BuyAsset,
    OpenedActive,
    BuyLpn,
    SellAsset,
    PaidActive,
    ClosingTransferIn,
    Closed,
    Liquidated,
}

const STATE_DB_ITEM: Item<'static, State> = Item::new("state");

pub(super) fn load(storage: &dyn Storage) -> ContractResult<State> {
    STATE_DB_ITEM.load(storage).map_err(Into::into)
}

#[cfg(feature = "migration")]
pub(super) fn load_v2(storage: &dyn Storage) -> ContractResult<StateV2> {
    Item::new("state").load(storage).map_err(Into::into)
}

pub(super) fn save(storage: &mut dyn Storage, next_state: &State) -> ContractResult<()> {
    STATE_DB_ITEM.save(storage, next_state).map_err(Into::into)
}

pub(super) fn new_lease(
    deps: &mut DepsMut<'_>,
    info: MessageInfo,
    spec: NewLeaseContract,
) -> ContractResult<(Batch, State)> {
    let (batch, start_state) = opening::request_loan::RequestLoan::new(deps, info, spec)?;
    Ok((batch, start_state.into()))
}

fn ignore_msg<S>(state: S) -> ContractResult<Response>
where
    S: Into<State>,
{
    Ok(Response::from(MessageResponse::default(), state))
}

mod impl_from {
    use super::{
        BuyAsset, BuyLpn, Closed, ClosingTransferIn, Liquidated, OpenedActive, PaidActive,
        RequestLoan, SellAsset, State,
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

    impl From<super::opened::liquidation::sell_asset::DexState> for State {
        fn from(value: super::opened::liquidation::sell_asset::DexState) -> Self {
            SellAsset::new(value).into()
        }
    }

    impl From<super::paid::Active> for State {
        fn from(value: super::paid::Active) -> Self {
            PaidActive::new(value).into()
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

/// would have used `enum_dispatch` it it supported trait associated types
mod impl_dex_handler {
    use dex::{ContinueResult, Handler, Result};
    use sdk::cosmwasm_std::{Binary, Deps, Env};

    use crate::error::ContractResult;

    use super::{Response, State};

    impl Handler for State {
        type Response = Self;
        type SwapResult = ContractResult<Response>;

        fn on_open_ica(
            self,
            counterparty_version: String,
            deps: Deps<'_>,
            env: Env,
        ) -> ContinueResult<Self> {
            match self {
                State::RequestLoan(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::OpenIcaAccount(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::BuyAsset(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::OpenedActive(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::BuyLpn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SellAsset(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::PaidActive(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::ClosingTransferIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::Closed(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::Liquidated(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
            }
        }

        fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::RequestLoan(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::OpenIcaAccount(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::BuyAsset(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::OpenedActive(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::BuyLpn(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::SellAsset(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::PaidActive(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::ClosingTransferIn(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::Closed(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::Liquidated(inner) => Handler::on_response(inner, data, deps, env).map_into(),
            }
        }

        fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_error(inner, deps, env),
                State::OpenIcaAccount(inner) => Handler::on_error(inner, deps, env),
                State::BuyAsset(inner) => Handler::on_error(inner, deps, env),
                State::OpenedActive(inner) => Handler::on_error(inner, deps, env),
                State::BuyLpn(inner) => Handler::on_error(inner, deps, env),
                State::SellAsset(inner) => Handler::on_error(inner, deps, env),
                State::PaidActive(inner) => Handler::on_error(inner, deps, env),
                State::ClosingTransferIn(inner) => Handler::on_error(inner, deps, env),
                State::Closed(inner) => Handler::on_error(inner, deps, env),
                State::Liquidated(inner) => Handler::on_error(inner, deps, env),
            }
        }

        fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_timeout(inner, deps, env),
                State::OpenIcaAccount(inner) => Handler::on_timeout(inner, deps, env),
                State::BuyAsset(inner) => Handler::on_timeout(inner, deps, env),
                State::OpenedActive(inner) => Handler::on_timeout(inner, deps, env),
                State::BuyLpn(inner) => Handler::on_timeout(inner, deps, env),
                State::SellAsset(inner) => Handler::on_timeout(inner, deps, env),
                State::PaidActive(inner) => Handler::on_timeout(inner, deps, env),
                State::ClosingTransferIn(inner) => Handler::on_timeout(inner, deps, env),
                State::Closed(inner) => Handler::on_timeout(inner, deps, env),
                State::Liquidated(inner) => Handler::on_timeout(inner, deps, env),
            }
        }

        fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::OpenIcaAccount(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::BuyAsset(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::OpenedActive(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::BuyLpn(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::SellAsset(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::PaidActive(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::ClosingTransferIn(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::Closed(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::Liquidated(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
            }
        }
    }
}
