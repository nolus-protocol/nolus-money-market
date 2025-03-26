use finance::duration::Duration;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

/// Contract during a DEX workflow
pub trait Contract
where
    Self: Sized,
{
    type StateResponse;

    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse;
}

/// Contract at a DEX stage
pub trait ContractInSwap
where
    Self: Sized,
{
    type StateResponse;

    fn state(
        self,
        in_progress: Stage,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse;
}

pub enum Stage {
    TransferOut,
    Swap,
    TransferInInit,
    TransferInFinish,
}
