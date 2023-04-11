use cosmwasm_std::Storage;
use enum_dispatch::enum_dispatch;
use platform::batch::Batch;
use serde::{Deserialize, Serialize};
use std::str;

use platform::message::Response as MessageResponse;
use sdk::{
    cosmwasm_std::{DepsMut, Env, MessageInfo, Reply},
    cw_storage_plus::Item,
};

use crate::{
    api::{ExecuteMsg, NewLeaseContract},
    error::ContractResult,
};

use self::{dex::State as DexState, lease::State as LeaseState};

pub(crate) use self::handler::{Handler, Response};

mod closed;
mod dex;
mod handler;
mod lease;
mod opened;
mod opening;
mod paid;

type RequestLoan = LeaseState<opening::request_loan::RequestLoan>;

type OpenIcaAccount = DexState<::dex::IcaConnector<opening::open_ica::OpenIcaAccount, SwapResult>>;

type BuyAsset = DexState<opening::buy_asset::DexState>;

type OpenedActive = LeaseState<opened::active::Active>;

type BuyLpn = DexState<opened::repay::buy_lpn::DexState>;

type PaidActive = LeaseState<paid::Active>;

type ClosingTransferIn = DexState<paid::transfer_in::DexState>;

type Closed = LeaseState<closed::Closed>;

type SwapResult = ContractResult<Response>;

#[enum_dispatch(Handler, Contract)]
#[derive(Serialize, Deserialize)]
pub(crate) enum State {
    RequestLoan,
    OpenIcaAccount,
    BuyAsset,
    OpenedActive,
    BuyLpn,
    PaidActive,
    ClosingTransferIn,
    Closed,
}

const STATE_DB_ITEM: Item<'static, State> = Item::new("state");

pub(super) fn load(storage: &dyn Storage) -> ContractResult<State> {
    STATE_DB_ITEM.load(storage).map_err(Into::into)
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
        BuyAsset, BuyLpn, Closed, ClosingTransferIn, OpenedActive, PaidActive, RequestLoan, State,
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

        #[inline]
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
                State::PaidActive(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::ClosingTransferIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::Closed(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
            }
        }

        #[inline]
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
                State::PaidActive(inner) => Handler::on_response(inner, data, deps, env).map_into(),
                State::ClosingTransferIn(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::Closed(inner) => Handler::on_response(inner, data, deps, env).map_into(),
            }
        }

        #[inline]
        fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_error(inner, deps, env),
                State::OpenIcaAccount(inner) => Handler::on_error(inner, deps, env),
                State::BuyAsset(inner) => Handler::on_error(inner, deps, env),
                State::OpenedActive(inner) => Handler::on_error(inner, deps, env),
                State::BuyLpn(inner) => Handler::on_error(inner, deps, env),
                State::PaidActive(inner) => Handler::on_error(inner, deps, env),
                State::ClosingTransferIn(inner) => Handler::on_error(inner, deps, env),
                State::Closed(inner) => Handler::on_error(inner, deps, env),
            }
        }

        #[inline]
        fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_timeout(inner, deps, env),
                State::OpenIcaAccount(inner) => Handler::on_timeout(inner, deps, env),
                State::BuyAsset(inner) => Handler::on_timeout(inner, deps, env),
                State::OpenedActive(inner) => Handler::on_timeout(inner, deps, env),
                State::BuyLpn(inner) => Handler::on_timeout(inner, deps, env),
                State::PaidActive(inner) => Handler::on_timeout(inner, deps, env),
                State::ClosingTransferIn(inner) => Handler::on_timeout(inner, deps, env),
                State::Closed(inner) => Handler::on_timeout(inner, deps, env),
            }
        }

        #[inline]
        fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::RequestLoan(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::OpenIcaAccount(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::BuyAsset(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::OpenedActive(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::BuyLpn(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::PaidActive(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::ClosingTransferIn(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::Closed(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
            }
        }
    }
}
