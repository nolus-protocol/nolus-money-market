use std::fmt::Display;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response as CwResponse};
use enum_dispatch::enum_dispatch;

use crate::{
    error::{ContractError as Err, ContractResult},
    msg::{ExecuteMsg, NewLeaseForm, StateQuery},
};

mod active;
mod no_lease;
mod no_lease_finish;
pub use active::Active;
pub use no_lease::NoLease;
pub use no_lease_finish::NoLeaseFinish;

#[enum_dispatch(Controller)]
pub enum State {
    NoLease,
    NoLeaseFinish,
    Active,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::NoLease(inner) => inner.fmt(f),
            State::NoLeaseFinish(inner) => inner.fmt(f),
            State::Active(inner) => inner.fmt(f),
        }
    }
}

pub type ExecuteResponse = Response<CwResponse>;
pub type QueryResponse = Response<Binary>;

pub struct Response<CwResp> {
    pub(super) cw_response: CwResp,
    pub(super) next_state: State,
}

impl<CwResp> Response<CwResp> {
    pub fn from<R, S>(resp: R, next_state: S) -> Self
    where
        R: Into<CwResp>,
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
    fn instantiate(
        self,
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _form: NewLeaseForm,
    ) -> ContractResult<ExecuteResponse> {
        err("instantiate", &self)
    }

    fn reply(self, _deps: DepsMut, _env: Env, _msg: Reply) -> ContractResult<ExecuteResponse> {
        err("reply", &self)
    }

    fn execute(
        self,
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<ExecuteResponse> {
        err("execute", &self)
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<QueryResponse> {
        err("query", &self)
    }
}

fn err<D, R>(op: &str, state: &D) -> ContractResult<R>
where
    D: Display,
{
    Err(Err::unsupported_operation(op, state))
}
