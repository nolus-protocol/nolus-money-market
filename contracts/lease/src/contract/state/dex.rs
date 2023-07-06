use serde::{Deserialize, Serialize};

use dex::{Contract as DexContract, Handler as DexHandler};
use platform::state_machine;
use sdk::cosmwasm_std::{Binary, Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{self, StateResponse},
    contract::api::Contract,
    error::ContractResult,
};

use super::{Response, State as ContractState};

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

impl<H> Contract for State<H>
where
    H: DexHandler<SwapResult = ContractResult<Response>>,
    H: DexContract<StateResponse = ContractResult<api::StateResponse>>,
    H::Response: Into<ContractState>,
    Self: Into<ContractState>,
{
    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler
            .on_open_ica(counterparty_version, deps, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn on_dex_response(self, data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.handler.on_response(data, deps, env).into()
    }

    fn on_dex_error(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.handler
            .on_error(deps, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn on_dex_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.handler
            .on_timeout(deps, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.handler.state(now, querier)
    }

    fn on_time_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_time_alarm(deps, env).into()
    }

    fn on_price_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
}
