#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response as CwResponse};

use crate::{
    contract::state::{Response, State},
    error::ContractResult,
    msg::{ExecuteMsg, NewLeaseForm, StateQuery},
};

use self::state::{Active, Controller, NoLease, NoLeaseFinish};

mod alarms;
mod close;
mod cmd;
mod open;
mod repay;
mod state;

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<CwResponse> {
    NoLease {}.instantiate(deps, env, info, form).map(|resp| {
        let Response {
            cw_response,
            next_state,
        } = resp;
        // TODO store the next_state
        debug_assert!(matches!(next_state, State::NoLease(_)));
        cw_response
    })
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    NoLeaseFinish {}.reply(deps, env, msg).map(|resp| {
        let Response {
            cw_response,
            next_state,
        } = resp;
        // TODO store the next_state
        debug_assert!(matches!(next_state, State::NoLeaseFinish(_)));
        cw_response
    })
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    Active {}.execute(deps, env, info, msg).map(|resp| {
        let Response {
            cw_response,
            next_state,
        } = resp;
        // TODO store the next_state
        debug_assert!(matches!(next_state, State::Active(_)));
        cw_response
    })
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: StateQuery) -> ContractResult<Binary> {
    Active {}.query(deps, env, msg).map(|resp| {
        let Response {
            cw_response,
            next_state,
        } = resp;
        // TODO store the next_state
        debug_assert!(matches!(next_state, State::Active(_)));
        cw_response
    })
}
