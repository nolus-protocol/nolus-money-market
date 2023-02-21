use cosmwasm_std::{QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo};

use crate::{
    api::{ExecuteMsg, StateResponse},
    contract::{Contract, Lease},
    error::ContractResult,
};

use super::{Controller, Response};

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
            ExecuteMsg::Repay() => todo!("fail"),
            ExecuteMsg::Close() => {
                let transfer_in = TransferInInit::new(self.lease);
                let batch = transfer_in.enter(deps.as_ref(), env)?;
                Ok(Response::from(batch, transfer_in))
            }
            ExecuteMsg::PriceAlarm() => {
                todo!("silently pass or make sure the alarm has been removed")
            }
            ExecuteMsg::TimeAlarm {} => {
                todo!("silently pass or make sure the alarm has been removed")
            }
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
