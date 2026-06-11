use dex::Enterable;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{contract::Lease, error::ContractResult};

use super::Response;

use self::transfer_out::DexState;
use cw_time::IntoInstant;

pub mod transfer_out;

pub fn start_close(
    lease: Lease,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    transfer_out::start(lease).and_then(|start_drain| {
        start_drain
            .enter(env.block.time.into_instant(), querier)
            .map(|drain_msgs| curr_request_response.merge_with(drain_msgs))
            .map(|batch| Response::from(batch, DexState::from(start_drain)))
            .map_err(Into::into)
    })
}
