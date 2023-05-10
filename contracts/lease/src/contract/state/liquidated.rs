use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{ExecuteMsg, StateResponse},
    contract::Contract,
    error::ContractResult,
};

use super::{handler, Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Liquidated {}

impl Handler for Liquidated {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => handler::err("repay", deps.api),
            ExecuteMsg::Close() => handler::err("close", deps.api),
            ExecuteMsg::PriceAlarm() | ExecuteMsg::TimeAlarm {} => super::ignore_msg(self),
        }
    }
}

impl Contract for Liquidated {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Liquidated())
    }
}
