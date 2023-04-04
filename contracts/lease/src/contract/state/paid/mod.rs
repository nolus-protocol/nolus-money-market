use serde::{Deserialize, Serialize};

use platform::response::response_with_messages;
use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{ExecuteMsg, StateResponse},
    contract::{Contract, Lease},
    error::ContractResult,
};

use super::{controller, Controller, Response};

use self::transfer_in_init::TransferInInit;

pub mod transfer_in_finish;
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
        deps: &mut DepsMut<'_>,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => controller::err("repay", deps.api),
            ExecuteMsg::Close() => {
                let transfer_in = TransferInInit::new(self.lease);

                transfer_in
                    .enter(env.block.time)
                    .and_then(|batch| {
                        response_with_messages(batch, &env.contract.address).map_err(Into::into)
                    })
                    .map(|response| Response::from(response, transfer_in))
            }
            ExecuteMsg::PriceAlarm() | ExecuteMsg::TimeAlarm {} => super::ignore_msg(&env, self),
        }
    }
}

impl Contract for Active {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: None,
        })
    }
}
