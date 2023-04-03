use serde::{Deserialize, Serialize};

use platform::{
    bank,
    batch::{Batch, Emit, Emitter},
};
use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{ExecuteMsg, StateResponse},
    contract::{cmd::Close, state, Contract},
    error::ContractResult,
    event::Type,
    lease::{with_lease, IntoDTOResult, LeaseDTO},
};

use super::{controller, Controller, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Closed {
    pub(super) fn enter_state(
        &self,
        lease: LeaseDTO,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Batch> {
        let lease_addr = lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        let IntoDTOResult {
            lease: _abandon,
            batch,
        } = with_lease::execute(lease, Close::new(lease_account), querier)?;
        Ok(batch)
    }

    pub(super) fn emit_ok(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl Controller for Closed {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        _env: &Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => controller::err("repay", deps.api),
            ExecuteMsg::Close() => controller::err("close", deps.api),
            ExecuteMsg::PriceAlarm() | ExecuteMsg::TimeAlarm {} => state::ignore_msg(self),
        }
    }
}

impl Contract for Closed {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
