use serde::{Deserialize, Serialize};

use dex::Enterable;
use sdk::cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{api::StateResponse, contract::Lease, error::ContractResult};

use super::{Handler, Response};

use self::transfer_in::DexState;

pub mod transfer_in;
#[cfg(feature = "migration")]
pub(super) mod v5;

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
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::paid_from(self.lease.lease, None))
    }

    fn close(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(&self.lease.lease.customer, &info.sender)?;

        let start_transfer_in = transfer_in::start(self.lease);
        start_transfer_in
            .enter(env.block.time, &deps.querier)
            .map(|batch| Response::from(batch, DexState::from(start_transfer_in)))
            .map_err(Into::into)
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
