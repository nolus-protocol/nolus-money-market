use cosmwasm_std::QuerierWrapper;
use enum_dispatch::enum_dispatch;
use sdk::cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Timestamp};

use crate::{
    api::StateResponse,
    error::{ContractError, ContractResult},
};

use super::state::Response;

// TODO consider merging it with crate::contract::Contract
// now this has a different name to workaround enum_dispatch failure to work with two distinct traits
// named the same way
#[enum_dispatch]
pub(super) trait ContractApi
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
