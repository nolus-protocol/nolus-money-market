use serde::{Deserialize, Serialize};

use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{paid::ClosingTrx, ExecuteMsg, StateResponse},
    contract::{
        state::{self, closed::Closed, controller, transfer_in, Controller, Response, State},
        Contract, Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_init::TransferInInit;

#[derive(Serialize, Deserialize)]
pub struct TransferInFinish {
    lease: Lease,
    timeout: Timestamp,
}

impl TransferInFinish {
    pub(super) fn new(lease: Lease, timeout: Timestamp) -> Self {
        Self { lease, timeout }
    }

    pub(super) fn try_complete(
        self,
        querier: &QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        let received =
            transfer_in::check_received(&self.lease.lease.amount, &env.contract.address, querier)?;

        let (next_state, cw_resp): (State, _) = if received {
            let closed = Closed::default();
            let emitter = closed.emit_ok(env, &self.lease.lease);
            let batch = closed.enter_state(self.lease.lease, querier)?;
            (closed.into(), batch.into_response(emitter))
        } else {
            let emitter = self.emit_ok();
            if env.block.time >= self.timeout {
                let back_to_init = TransferInInit::new(self.lease);
                let batch = back_to_init.enter(env.block.time)?;
                (back_to_init.into(), batch.into_response(emitter))
            } else {
                let batch =
                    transfer_in::setup_alarm(self.lease.lease.time_alarms.clone(), env.block.time)?;
                (self.into(), batch.into_response(emitter))
            }
        };
        Ok(Response::from(cw_resp, next_state))
    }

    fn on_alarm(self, querier: &QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        self.try_complete(querier, env)
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::ClosingTransferIn)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.lease.lease.amount.clone())
    }
}

impl Controller for TransferInFinish {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => controller::err("repay", deps.api),
            ExecuteMsg::Close() => controller::err("close", deps.api),
            ExecuteMsg::PriceAlarm() => state::ignore_msg(self)?.attach_alarm_response(&env),
            ExecuteMsg::TimeAlarm {} => self
                .on_alarm(&deps.querier, &env)?
                .attach_alarm_response(&env),
        }
    }
}

impl Contract for TransferInFinish {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: Some(ClosingTrx::TransferInFinish),
        })
    }
}
