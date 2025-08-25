use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{api::query::StateResponse, error::ContractResult};

use super::{Handler, Response, drain::DrainAll};

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Handler for Closed {
    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }

    fn on_time_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }

    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.drain(&env.contract.address, info.sender, querier)
    }
}

impl DrainAll for Closed {}
