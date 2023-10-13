use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{api::StateResponse, error::ContractResult};

use super::{Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Handler for Closed {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }

    fn on_time_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
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
