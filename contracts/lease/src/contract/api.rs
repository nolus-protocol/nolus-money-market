use enum_dispatch::enum_dispatch;

use sdk::cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Timestamp,
};

use crate::{
    api::StateResponse,
    error::{ContractError, ContractResult},
};

use super::state::Response;

#[enum_dispatch]
pub(super) trait Contract
where
    Self: Sized,
{
    fn on_open_ica(
        self,
        _counterparty_version: String,
        _deps: Deps<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err("open ica response")
    }

    fn on_dex_response(
        self,
        _response: Binary,
        _deps: Deps<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err("dex response")
    }

    fn on_dex_error(self, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("dex error")
    }

    fn on_dex_timeout(self, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("dex timeout")
    }

    /// The inner entry point for safe delivery of a Dex response
    ///
    /// The aim is to plug another level in the Cosmwasm messages tree. That allows the code
    /// to handle errors that might occur in the sub-messages, not only in the main one.
    /// Cosmwasm guarantees that it would call `reply` when the sub-message is scheduled
    /// with the correct flag, e.g. ReplyOn::Error or ReplyOn::Always.
    /// Intended to be invoked always by the same contract instance.
    /// The anticipated execution flow, for example when delivering a Dex response, is
    /// `on_dex_response`, `on_dex_inner`, sub-message-1, ... sub-message-N, `reply`
    fn on_dex_inner(self, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("dex inner")
    }

    /// The inner entry point for safe delivery of a ICA Open response, error or timeout
    ///
    /// The aim is to plug another level in the Cosmwasm messages tree. That allows the code
    /// to handle errors that might occur in the sub-messages, not only in the main one.
    /// Cosmwasm guarantees that it would call `reply` when the sub-message is scheduled
    /// with the correct flag, e.g. ReplyOn::Error or ReplyOn::Always.
    /// Intended to be invoked always by the same contract instance.
    /// The anticipated execution flow, for example when delivering a Dex response, is
    /// `on_dex_response`, `on_dex_inner`, sub-message-1, ... sub-message-N, `reply`
    fn on_dex_inner_continue(self, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("dex inner continue")
    }

    fn heal(self, _deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("heal")
    }

    fn is_finished(&self) -> bool;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse>;

    fn reply(self, _deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply")
    }

    fn repay(
        self,
        _deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("repay")
    }

    fn close(
        self,
        _deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("close")
    }

    fn on_time_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on time alarm")
    }

    fn on_price_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on price alarm")
    }
}

fn err<R>(op: &str) -> ContractResult<R> {
    Err(ContractError::unsupported_operation(op))
}
