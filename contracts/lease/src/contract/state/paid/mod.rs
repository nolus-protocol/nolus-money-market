use serde::{Deserialize, Serialize};

use dex::Enterable;
use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{ExecuteMsg, StateResponse},
    contract::{Contract, Lease},
    error::{ContractError, ContractResult},
};

use super::{handler, Handler, Response};

use self::transfer_in::DexState;

pub mod transfer_in;
#[cfg(feature = "migration")]
pub mod v2;

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }
}

impl Handler for Active {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => handler::err("repay", deps.api),
            ExecuteMsg::Close() => {
                if self.lease.lease.customer != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
                let start_transfer_in = transfer_in::start(self.lease);
                start_transfer_in
                    .enter(env.block.time, &deps.querier)
                    .map(|batch| Response::from(batch, DexState::from(start_transfer_in)))
                    .map_err(Into::into)
            }
            ExecuteMsg::PriceAlarm() | ExecuteMsg::TimeAlarm {} => super::ignore_msg(self),
        }
    }
}

impl Contract for Active {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::paid_from(self.lease.lease, None))
    }
}
