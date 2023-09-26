use currency::Currency;

use crate::{
    coin::Coin,
    error::Result,
    fraction::Fraction,
    percent::Percent,
    price::{total_of, Price},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub enum Level {
    First(Percent),
    Second(Percent),
    Third(Percent),
    Max(Percent),
}

impl Level {
    pub fn ltv(&self) -> Percent {
        *match self {
            Self::First(ltv) | Self::Second(ltv) | Self::Third(ltv) | Self::Max(ltv) => ltv,
        }
    }

    pub fn ordinal(self) -> u8 {
        match self {
            Self::First(_) => 1,
            Self::Second(_) => 2,
            Self::Third(_) => 3,
            Self::Max(_) => 4,
        }
    }

    pub fn price_alarm<Asset, Lpn>(
        &self,
        amount: Coin<Asset>,
        liability: Coin<Lpn>,
    ) -> Result<Price<Asset, Lpn>>
    where
        Asset: Currency,
        Lpn: Currency,
    {
        debug_assert!(
            !liability.is_zero(),
            "Loan already paid, no need of next alarms!"
        );
        debug_assert!(!self.ltv().is_zero());

        Ok(total_of(self.ltv().of(amount)).is(liability))
    }
}

impl From<Level> for Percent {
    fn from(value: Level) -> Self {
        value.ltv()
    }
}
