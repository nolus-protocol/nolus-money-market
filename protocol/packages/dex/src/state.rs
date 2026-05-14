use finance::duration::Duration;
use finance::instant::Instant;
use sdk::cosmwasm_std::QuerierWrapper;

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

pub enum Stage {
    TransferOut,
    Swap,
    TransferInInit,
    TransferInFinish,
}
