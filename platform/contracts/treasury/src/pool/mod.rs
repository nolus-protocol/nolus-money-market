use finance::{duration::Duration, percent::Percent100};
use lpp_platform::CoinStable;
use platform::message::Response as MessageResponse;

use crate::ContractError;

pub use impl_::Pool as PoolImpl;

mod impl_;
#[cfg(test)]
pub mod mock;

pub trait Pool {
    fn balance(&self) -> CoinStable;

    fn distribute_rewards(
        self,
        apr: Percent100,
        period: Duration,
    ) -> Result<MessageResponse, ContractError>;
}
