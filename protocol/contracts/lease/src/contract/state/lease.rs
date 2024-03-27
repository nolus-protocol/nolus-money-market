use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Reply, Timestamp};

use crate::{
    api::{position::PositionClose, query::StateResponse},
    error::ContractResult,
};

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

    pub fn map<MapFn, HTo>(self, map_fn: MapFn) -> State<HTo>
    where
        MapFn: FnOnce(H) -> HTo,
    {
        State::new(map_fn(self.handler))
    }
}

impl<H> Contract for State<H>
where
    H: LeaseHandler,
{
    fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.handler.state(now, querier)
    }

    fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.handler.reply(querier, env, msg)
    }

    fn repay(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.repay(querier, env, info)
    }

    fn close_position(
        self,
        spec: PositionClose,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.close_position(spec, querier, env, info)
    }

    fn close(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.close(querier, env, info)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_time_alarm(querier, env, info)
    }

    fn on_price_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_price_alarm(querier, env, info)
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> ContractResult<Response> {
        self.handler.heal(querier, env)
    }
}
