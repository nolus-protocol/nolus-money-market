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

    pub(super) fn reward_scale<Lpn, Lpp>(&self, lpp: &Lpp) -> ContractResult<ActiveRewardScale<Lpn>>
    where
        Lpn: Currency,
        Lpp: LppTrait<Lpn>,
    {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let resp = lpp.lpp_balance()?;
        let tvl: Coin<Lpn> = resp.balance + resp.total_principal_due + resp.total_interest_due;

        // get annual percentage of return from configuration
        Ok(ActiveRewardScale {
            tvl,
            apr: self.scale.get_apr(tvl.into()),
        })
    }
}

pub(super) struct ActiveRewardScale<Lpn>
where
    Lpn: Currency,
{
    pub tvl: Coin<Lpn>,
    pub apr: Percent,
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
            .map(|ActiveRewardScale { apr, .. }| apr)
    }
}
