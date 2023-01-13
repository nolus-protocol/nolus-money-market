use std::fmt::Display;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{ExecuteMsg, StateQuery, StateResponse},
    error::{ContractError as Err, ContractResult},
};

pub use self::{
    active::Active, buy_asset::BuyAsset, open_ica_account::OpenIcaAccount,
    request_loan::RequestLoan, transfer_out::TransferOut,
};

mod active;
mod buy_asset;
mod open_ica_account;
mod request_loan;
mod transfer_out;

#[enum_dispatch(Controller)]
#[derive(Serialize, Deserialize)]
pub enum State {
    RequestLoan,
    OpenIcaAccount,
    TransferOut,
    BuyAsset,
    Active,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            State::RequestLoan(inner) => inner.fmt(f),
            State::OpenIcaAccount(inner) => inner.fmt(f),
            State::TransferOut(inner) => inner.fmt(f),
            State::BuyAsset(inner) => inner.fmt(f),
            State::Active(inner) => inner.fmt(f),
        }
    }
}

pub struct Response {
    pub(super) cw_response: CwResponse,
    pub(super) next_state: State,
}

impl Response {
    pub fn from<R, S>(resp: R, next_state: S) -> Self
    where
        R: Into<CwResponse>,
        S: Into<State>,
    {
        Self {
            cw_response: resp.into(),
            next_state: next_state.into(),
        }
    }
}

#[enum_dispatch]
pub trait Controller
where
    Self: Sized,
    Self: Display,
{
    fn reply(self, _deps: &mut DepsMut, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply", &self)
    }

    fn execute(
        self,
        _deps: &mut DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        err("execute", &self)
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        err("query", &self)
    }

    fn sudo(self, _deps: &mut DepsMut, _env: Env, _msg: SudoMsg) -> ContractResult<Response> {
        err("sudo", &self)
    }
}

fn err<D, R>(op: &str, state: &D) -> ContractResult<R>
where
    D: Display,
{
    Err(Err::unsupported_operation(op, state))
}
