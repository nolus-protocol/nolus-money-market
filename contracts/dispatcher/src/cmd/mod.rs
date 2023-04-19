use currency::{lpn::Lpns, native::Native};
use finance::coin::CoinDTO;
use platform::batch::Batch;

pub use self::{dispatch::Dispatch, query_reward_scale::QueryRewardScale};

mod dispatch;
mod query_reward_scale;

pub struct Result {
    pub batch: Batch,
    pub receipt: Receipt,
}

#[derive(Eq, PartialEq)]
pub struct Receipt {
    pub in_stable: CoinDTO<Lpns>,
    pub in_nls: CoinDTO<Native>,
}
