use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::{FullClose, PartialClose},
    contract::{state::Response, Lease},
    error::ContractResult,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

// TODO abstract LiquidationDTO-to-ClosePositionTask to avoid match-ing PositionClose variants

#[allow(unused)]
fn start_partial(
    close: PartialClose,
    lease: Lease,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    //TODO validate the close.amount against the lease position using a cmd
    partial::RepayableImpl::from(close).start(lease, MessageResponse::default(), env, querier)
}

#[allow(unused)]
fn start_full(
    close: FullClose,
    lease: Lease,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    full::RepayableImpl::from(close).start(lease, MessageResponse::default(), env, querier)
}
