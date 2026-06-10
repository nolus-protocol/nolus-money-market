use finance::duration::Duration;
use finance::instant::Instant;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::swap_task::CoinsNb;

/// Contract during a DEX workflow
pub trait Contract
where
    Self: Sized,
{
    type StateResponse;

    fn state(
        self,
        now: Instant,
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
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse;
}

/// Contract during a remote swap leg sequence
///
/// The remote-swap counterpart of [`ContractInSwap`]. The progress is
/// expressed as the number of swap legs still awaiting an acknowledgment.
pub trait ContractInRemoteSwap
where
    Self: Sized,
{
    type StateResponse;

    fn state(
        self,
        acks_left: CoinsNb,
        now: Instant,
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
