use enum_dispatch::enum_dispatch;

use platform::state_machine::Response as StateMachineResponse;
use sdk::cosmwasm_std::{Api, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Timestamp};

use crate::{
    api::StateResponse,
    error::{ContractError, ContractResult},
};

use super::State;

pub(crate) type Response = StateMachineResponse<State>;
#[enum_dispatch]
pub(crate) trait Handler
where
    Self: Sized,
{
    const IS_FINISHED: bool;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse>;

    fn reply(self, deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply", deps.api)
    }

    fn repay(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("repay", deps.api)
    }

    fn close(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("close", deps.api)
    }

    fn on_time_alarm(
        self,
        deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on time alarm", deps.api)
    }

    fn on_price_alarm(
        self,
        deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on price alarm", deps.api)
    }

    fn heal(self, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("heal", deps.api)
    }
}

fn err<R>(op: &str, api: &dyn Api) -> ContractResult<R> {
    let err = ContractError::unsupported_operation(op);
    api.debug(&format!("{:?}", op));

    Err(err)
}
