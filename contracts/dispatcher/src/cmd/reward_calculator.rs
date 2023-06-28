use currency::Currency;
use finance::{coin::Coin, percent::Percent};
use lpp::{
    msg::LppBalanceResponse,
    stub::{Lpp as LppTrait, WithLpp},
};

use crate::{result::ContractResult, state::reward_scale::RewardScale, ContractError};

pub struct RewardCalculator<'r> {
    scale: &'r RewardScale,
}

impl<'r> RewardCalculator<'r> {
    pub fn new(scale: &'r RewardScale) -> Self {
        Self { scale }
    }

    pub(super) fn calculate<Lpn, Lpp>(&self, lpp: &Lpp) -> ContractResult<Reward<Lpn>>
    where
        Lpn: Currency,
        Lpp: LppTrait<Lpn>,
    {
        lpp.lpp_balance()
            .map(
                |LppBalanceResponse {
                     balance,
                     total_principal_due,
                     total_interest_due,
                     ..
                 }| balance + total_principal_due + total_interest_due,
            )
            .map(|tvl: Coin<Lpn>| Reward {
                tvl,
                apr: self.scale.get_apr(tvl.into()),
            })
            .map_err(Into::into)
    }
}

pub(super) struct Reward<Lpn>
where
    Lpn: Currency,
{
    pub tvl: Coin<Lpn>,
    pub apr: Percent,
}

impl<'r> WithLpp for RewardCalculator<'r> {
    type Output = Percent;
    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpp: LppTrait<Lpn>,
    {
        self.calculate(&lpp).map(|Reward { apr, .. }| apr)
    }
}
