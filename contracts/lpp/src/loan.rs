use std::cmp;

use cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};

use finance::{
    coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod, percent::Percent,
};
use sdk::schemars::{self, JsonSchema};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
#[serde(rename_all = "snake_case")]
pub struct LoanData<Lpn>
where
    Lpn: Currency,
{
    pub principal_due: Coin<Lpn>,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

impl<Lpn> LoanData<Lpn>
where
    Lpn: Currency,
{
    pub fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
        let delta_t = Duration::between(self.interest_paid, cmp::max(by, self.interest_paid));

        let interest_period = InterestPeriod::with_interest(self.annual_interest_rate)
            .from(self.interest_paid)
            .spanning(delta_t);

        interest_period.interest(self.principal_due)
    }
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin, duration::Duration, percent::Percent, test::currency::Usdc, zero::Zero,
    };
    use sdk::cosmwasm_std::Timestamp;

    use crate::loan::LoanData;

    #[test]
    fn interest() {
        let l = LoanData {
            principal_due: Coin::<Usdc>::from(100),
            annual_interest_rate: Percent::from_percent(50),
            interest_paid: Timestamp::from_nanos(200),
        };

        assert_eq!(
            Coin::<Usdc>::from(50),
            l.interest_due(l.interest_paid + Duration::YEAR)
        );

        assert_eq!(Coin::ZERO, l.interest_due(l.interest_paid));
        assert_eq!(Coin::ZERO, l.interest_due(l.interest_paid.minus_nanos(1)));
    }
}
