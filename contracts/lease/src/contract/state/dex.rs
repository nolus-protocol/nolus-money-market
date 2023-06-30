use serde::{Deserialize, Serialize};

use dex::{ContinueResult, Contract as DexContract, Handler as DexHandler, Result as DexResult};
use platform::state_machine;
use sdk::cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{self, ExecuteMsg, StateResponse},
    contract::Contract,
    error::ContractResult,
};

use super::{
    handler::{self, Handler as LeaseHandler},
    Response, State as ContractState,
};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct State<H> {
    handler: H,
}

impl<H> State<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<H> LeaseHandler for State<H>
where
    H: DexHandler<SwapResult = ContractResult<Response>>,
    H::Response: Into<ContractState>,
    Self: Into<ContractState>,
{
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::TimeAlarm {} => DexHandler::on_time_alarm(self, deps.as_ref(), env).into(),
            ExecuteMsg::PriceAlarm {} => super::ignore_msg(self),
            _ => handler::err("execute", deps.api),
        }
    }
}

impl<H> DexHandler for State<H>
where
    H: DexHandler<SwapResult = ContractResult<Response>>,
    H::Response: Into<ContractState>,
{
    type Response = ContractState;
    type SwapResult = H::SwapResult;

    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContinueResult<Self> {
        self.handler
            .on_open_ica(counterparty_version, deps, env)
            .map(state_machine::from)
    }

    fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        self.handler.on_response(data, deps, env).map_into()
    }

    fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        self.handler.on_error(deps, env).map(state_machine::from)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        self.handler.on_timeout(deps, env).map(state_machine::from)
    }

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        self.handler.on_time_alarm(deps, env).map_into()
    }
}

impl<H> Contract for State<H>
where
    H: DexContract<StateResponse = ContractResult<api::StateResponse>>,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.handler.state(now, querier)
    }
}
