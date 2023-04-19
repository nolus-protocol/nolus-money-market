use currency::{lpn::Lpns, native::Native};
use finance::coin::CoinDTO;
use platform::batch::Batch;

pub use self::{dispatch::Dispatch, reward_calculator::RewardCalculator};

mod dispatch;
mod reward_calculator;

pub struct Result {
    pub batch: Batch,
    pub receipt: Receipt,
}

#[derive(Eq, PartialEq)]
pub struct Receipt {
    pub in_stable: CoinDTO<Lpns>,
    pub in_nls: CoinDTO<Native>,
}
