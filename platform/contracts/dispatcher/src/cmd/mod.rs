use currency::NlsPlatform;
use finance::coin::Coin;
use lpp_platform::Usd;
use platform::batch::Batch;

pub(crate) use self::{
    dispatch::Dispatch,
    reward_calculator::{Reward, RewardCalculator},
};

mod dispatch;
mod reward_calculator;

pub struct Result {
    pub batch: Batch,
    pub receipt: Receipt,
}

#[derive(Eq, PartialEq)]
pub struct Receipt {
    pub in_stable: Coin<Usd>,
    pub in_nls: Coin<NlsPlatform>,
}
