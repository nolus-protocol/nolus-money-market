use enum_dispatch::enum_dispatch;

use platform::state_machine::Response as StateMachineResponse;
use sdk::cosmwasm_std::{Api, DepsMut, Env, MessageInfo, Reply};

use crate::{
    api::ExecuteMsg,
    error::{ContractError, ContractResult},
};

use super::State;

pub(crate) type Response = StateMachineResponse<State>;
#[enum_dispatch]
pub(crate) trait Handler
where
    Self: Sized,
{
    fn reply(self, deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply", deps.api)
    }

    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        err("execute", deps.api)
    }
}

pub(super) fn err<R>(op: &str, api: &dyn Api) -> ContractResult<R> {
    let err = ContractError::unsupported_operation(op);
    api.debug(&format!("{:?}", op));

    Err(err)
}
