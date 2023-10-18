use finance::{coin::Coin, percent::Percent};
use lpp_platform::{Lpp as LppTrait, LppBalanceResponse, Usd};

use crate::{result::ContractResult, state::reward_scale::RewardScale};

pub struct RewardCalculator<'r> {
    scale: &'r RewardScale,
}

impl<'r> RewardCalculator<'r> {
    pub fn new(scale: &'r RewardScale) -> Self {
        Self { scale }
    }

    pub fn calculate<Lpp>(&self, lpp: &Lpp) -> ContractResult<Reward>
    where
        Lpp: LppTrait,
    {
        lpp.balance()
            .map(
                |LppBalanceResponse {
                     balance,
                     total_principal_due,
                     total_interest_due,
                     ..
                 }| balance + total_principal_due + total_interest_due,
            )
            .map(|tvl| Reward {
                tvl,
                apr: self.scale.get_apr(tvl.into()),
            })
            .map_err(Into::into)
    }
}

pub struct Reward {
    pub tvl: Coin<Usd>,
    pub apr: Percent,
}
