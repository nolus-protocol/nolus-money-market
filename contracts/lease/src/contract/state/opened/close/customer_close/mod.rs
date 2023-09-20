use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::LeaseCoin,
    contract::{state::Response, Lease},
    error::ContractResult,
};

pub mod full;
pub mod partial;

#[allow(unused)]
fn start_partial(
    amount: LeaseCoin,
    lease: Lease,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    super::start_impl::<_, partial::RepayableImpl>(
        lease,
        partial::PartialClose::new(amount),
        MessageResponse::default(),
        env,
        querier,
    )
}

#[allow(unused)]
fn start_full(lease: Lease, env: &Env, querier: &QuerierWrapper<'_>) -> ContractResult<Response> {
    super::start_impl::<_, full::RepayableImpl>(
        lease,
        full::FullClose(),
        MessageResponse::default(),
        env,
        querier,
    )
}
