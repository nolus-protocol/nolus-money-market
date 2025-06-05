use serde::{Deserialize, Serialize};

use dex::Enterable;
use finance::duration::Duration;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{api::query::StateResponse, contract::Lease, error::ContractResult};

use super::{Handler, Response};

use self::transfer_in::DexState;

pub mod transfer_in;

pub fn start_close(
    lease: Lease,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let start_transfer_in = transfer_in::start(lease);
    start_transfer_in
        .enter(env.block.time, querier)
        .map(|close_msgs| curr_request_response.merge_with(close_msgs))
        .map(|batch| Response::from(batch, DexState::from(start_transfer_in)))
        .map_err(Into::into)
}

//TODO remove it once all leases have been gone away from this state - by their owners or by the `heal` caller
#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Handler for Active {
    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::paid_from(self.lease.lease, None))
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
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        start_close(self.lease, MessageResponse::default(), &env, querier)
    }
}
