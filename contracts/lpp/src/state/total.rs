use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::Currency;
use finance::{
    coin::Coin, fraction::Fraction, interest::InterestPeriod, percent::Percent, period::Period,
    ratio::Rational,
};
use sdk::{
    cosmwasm_std::{StdResult, Storage, Timestamp},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct Total<Lpn>
where
    Lpn: Currency,
{
    total_principal_due: Coin<Lpn>,
    total_interest_due: Coin<Lpn>,
    annual_interest_rate: Rational<Coin<Lpn>>,
    last_update_time: Timestamp,
}

impl<Lpn> Default for Total<Lpn>
where
    Lpn: Currency + Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Lpn> Total<Lpn>
where
    Lpn: Currency + Serialize + DeserializeOwned,
{
    const STORAGE: Item<'static, Total<Lpn>> = Item::new("total");

    pub fn new() -> Self {
        Total {
            total_principal_due: Coin::new(0),
            total_interest_due: Coin::new(0),
            annual_interest_rate: Rational::new(Coin::new(0), Coin::new(1000)),
            last_update_time: Timestamp::default(),
        }
    }

    pub fn total_principal_due(&self) -> Coin<Lpn> {
        self.total_principal_due
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn total_interest_due_by_now(&self, ctime: Timestamp) -> Coin<Lpn> {
        InterestPeriod::<Coin<Lpn>, _>::with_interest(self.annual_interest_rate)
            .and_period(Period::from_till(self.last_update_time, ctime))
            .interest(self.total_principal_due)
            + self.total_interest_due
    }

    pub fn borrow(
        &mut self,
        ctime: Timestamp,
        amount: Coin<Lpn>,
        loan_interest_rate: Percent,
    ) -> Result<&Self, ContractError> {
        self.total_interest_due = self.total_interest_due_by_now(ctime);

        // TODO: get rid of fully qualified syntax
        self.annual_interest_rate = Rational::new(
            Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                + loan_interest_rate.of(amount),
            self.total_principal_due + amount,
        );

        self.total_principal_due += amount;

        self.last_update_time = ctime;

        Ok(self)
    }

    pub fn repay(
        &mut self,
        ctime: Timestamp,
        loan_interest_payment: Coin<Lpn>,
        loan_principal_payment: Coin<Lpn>,
        loan_interest_rate: Percent,
    ) -> &Self {
        self.total_interest_due = self.total_interest_due_by_now(ctime) - loan_interest_payment;

        self.annual_interest_rate = if self.total_principal_due == loan_principal_payment {
            Rational::new(Coin::<Lpn>::new(0), Coin::<Lpn>::new(100))
        } else {
            Rational::new(
                Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                    - loan_interest_rate.of(loan_principal_payment),
                self.total_principal_due - loan_principal_payment,
            )
        };

        self.total_principal_due -= loan_principal_payment;

        self.last_update_time = ctime;

        self
    }
}

#[cfg(test)]
mod test {
    use currency::test::Usdc;
    use finance::duration::Duration;
    use sdk::cosmwasm_std::testing;

    use super::*;

    #[test]
    fn borrow_and_repay() {
        let mut deps = testing::mock_dependencies();
        let mut env = testing::mock_env();

        let total: Total<Usdc> = Total::default();
        total.store(deps.as_mut().storage).expect("should store");

        let mut total: Total<Usdc> = Total::load(deps.as_ref().storage).expect("should load");

        assert_eq!(total.total_principal_due(), Coin::<Usdc>::new(0));

        total
            .borrow(env.block.time, Coin::new(10000), Percent::from_percent(20))
            .expect("should borrow");
        assert_eq!(total.total_principal_due(), Coin::new(10000));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(env.block.time);
        assert_eq!(interest_due, Coin::new(1000));

        total.repay(
            env.block.time,
            Coin::new(1000),
            Coin::new(5000),
            Percent::from_percent(20),
        );
        assert_eq!(total.total_principal_due(), Coin::new(5000));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(env.block.time);
        assert_eq!(interest_due, 500u128.into());
    }
}
