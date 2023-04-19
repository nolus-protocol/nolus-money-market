use finance::{coin::Coin, currency::Currency, percent::Percent};
use lpp::stub::{Lpp as LppTrait, WithLpp};

use crate::{result::ContractResult, state::reward_scale::RewardScale, ContractError};

pub struct QueryRewardScale<'r> {
    scale: &'r RewardScale,
}

impl<'r> QueryRewardScale<'r> {
    pub fn new(scale: &'r RewardScale) -> Self {
        Self { scale }
    }

    fn reward_scale<Lpn, Lpp>(&self, lpp: &Lpp) -> ContractResult<Percent>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let resp = lpp.lpp_balance()?;
        let tvl: Coin<Lpn> = resp.balance + resp.total_principal_due + resp.total_interest_due;

        // get annual percentage of return from configuration
        Ok(self.scale.get_apr(tvl.into()))
    }
}

impl<'r> WithLpp for QueryRewardScale<'r> {
    type Output = Percent;
    type Error = ContractError;

    #[inline]
    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpp: LppTrait<Lpn>,
    {
        self.reward_scale(&lpp)
    }
}
