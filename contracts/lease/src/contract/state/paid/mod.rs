use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo};
use serde::{Deserialize, Serialize};

use crate::{
    api::{ExecuteMsg, StateQuery, StateResponse},
    contract::Lease,
    error::ContractResult,
};

use self::transfer_in_init::TransferInInit;

use super::{Controller, Response};

pub mod transfer_in_init;

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }
}

impl Controller for Active {
    fn execute(
        self,
        _deps: &mut DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => todo!("fail"),
            ExecuteMsg::Close() => {
                let next_state = TransferInInit::new(self.lease);
                let batch = next_state.enter_state(env.block.time)?;
                Ok(Response::from(batch, next_state))
            }
            ExecuteMsg::PriceAlarm() => {
                todo!("silently pass or make sure the alarm has been removed")
            }
            ExecuteMsg::TimeAlarm(_block_time) => {
                todo!("silently pass or make sure the alarm has been removed")
            }
        }
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: None,
        })
    }
}
