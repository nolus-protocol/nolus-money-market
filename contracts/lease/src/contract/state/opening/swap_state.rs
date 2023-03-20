use cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{api::StateResponse, error::ContractError};

pub(super) struct TransferOutState {}
pub(super) struct SwapState {}

pub(super) trait ContractInSwap<State>
where
    Self: Sized,
{
    fn state(
        self,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> Result<StateResponse, ContractError>;
}
