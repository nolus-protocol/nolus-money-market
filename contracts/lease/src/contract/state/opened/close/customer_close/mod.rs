use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::PositionClose,
    contract::{state::Response, Lease},
    error::ContractResult,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

// TODO abstract LiquidationDTO-to-ClosePositionTask to avoid match-ing PositionClose variants
pub(in crate::contract::state::opened) fn start(
    close: PositionClose,
    lease: Lease,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    // TODO abstract MarketClose-to-ClosePositionTask to avoid this match
    match close {
        PositionClose::PartialClose(spec) =>
        //TODO validate the close.amount against the lease position using a cmd
        {
            partial::RepayableImpl::from(spec).start(
                lease,
                MessageResponse::default(),
                env,
                querier,
            )
        }
        PositionClose::FullClose(spec) => {
            full::RepayableImpl::from(spec).start(lease, MessageResponse::default(), env, querier)
        }
    }
}
