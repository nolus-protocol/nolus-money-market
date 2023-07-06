use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Timestamp};

use crate::{api::StateResponse, error::ContractResult};

use super::{handler::Handler as LeaseHandler, Contract, Response};

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
    H: LeaseHandler,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.handler.state(now, querier)
    }

    fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.handler.reply(deps, env, msg)
    }

    fn repay(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.repay(deps, env, info)
    }

    fn close(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.close(deps, env, info)
    }

    fn on_time_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_time_alarm(deps, env, info)
    }

    fn on_price_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_price_alarm(deps, env, info)
    }
}
