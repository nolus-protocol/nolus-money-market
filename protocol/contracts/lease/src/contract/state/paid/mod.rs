use dex::Enterable;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{contract::Lease, error::ContractResult};

use super::Response;

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
