use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    interest,
    percent::Percent,
    ratio::Rational,
    zero::Zero,
};
use sdk::{
    cosmwasm_std::{StdResult, Storage, Timestamp},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Total<Lpn>
where
    Lpn: ?Sized,
{
    total_principal_due: Coin<Lpn>,
    total_interest_due: Coin<Lpn>,
    annual_interest_rate: Rational<Coin<Lpn>>,
    last_update_time: Timestamp,
}

impl<Lpn> Default for Total<Lpn>
where
    Lpn: ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Lpn> Total<Lpn>
where
    Lpn: ?Sized,
{
    const STORAGE: Item<'static, Total<Lpn>> = Item::new("total");

    pub fn new() -> Self {
        Total {
            total_principal_due: Coin::ZERO,
            total_interest_due: Coin::ZERO,
            annual_interest_rate: zero_interest_rate(),
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

    pub fn total_interest_due_by_now(&self, ctime: &Timestamp) -> Coin<Lpn> {
        interest::interest::<Coin<Lpn>, _, _>(
            self.annual_interest_rate,
            self.total_principal_due,
            Duration::between(&self.last_update_time, ctime),
        ) + self.total_interest_due
    }

    pub fn borrow(
        &mut self,
        ctime: Timestamp,
        amount: Coin<Lpn>,
        loan_interest_rate: Percent,
    ) -> Result<&Self, ContractError> {
        self.total_interest_due = self.total_interest_due_by_now(&ctime);

        let updated_total_principal_due =
            self.total_principal_due
                .checked_add(amount)
                .ok_or(ContractError::OverflowError(
                    "Total principal due overflow".to_string(),
                ))?;

        // TODO: get rid of fully qualified syntax
        self.annual_interest_rate = Rational::new(
            Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                .checked_add(loan_interest_rate.of(amount))
                .ok_or(ContractError::OverflowError(
                    "Annual interest rate calculation overflow".to_string(),
                ))?,
            updated_total_principal_due,
        );

        self.total_principal_due = updated_total_principal_due;

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
        self.total_interest_due = self
            .total_interest_due_by_now(&ctime)
            .saturating_sub(loan_interest_payment);

        if loan_principal_payment >= self.total_principal_due {
            self.annual_interest_rate = zero_interest_rate();
            self.total_principal_due = Coin::ZERO;
        } else {
            self.annual_interest_rate = Rational::new(
                Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                    - loan_interest_rate.of(loan_principal_payment),
                self.total_principal_due - loan_principal_payment,
            );
            self.total_principal_due -= loan_principal_payment;
        }

        self.last_update_time = ctime;

        self
    }
}

fn zero_interest_rate<Lpn>() -> Rational<Coin<Lpn>>
where
    Lpn: ?Sized,
{
    const THOUSAND: Amount = 1000;
    Rational::new(Coin::ZERO, THOUSAND.into())
}

#[cfg(test)]
mod test {
    use currencies::test::LpnC;
    use finance::duration::Duration;
    use sdk::cosmwasm_std::testing;

    use super::*;

    #[test]
    fn borrow_and_repay() {
        let mut deps = testing::mock_dependencies();
        let mut env = testing::mock_env();

        let total: Total<LpnC> = Total::default();
        total.store(deps.as_mut().storage).expect("should store");

        let mut total: Total<LpnC> = Total::load(deps.as_ref().storage).expect("should load");

        assert_eq!(total.total_principal_due(), Coin::<LpnC>::new(0));

        total
            .borrow(env.block.time, Coin::new(10000), Percent::from_percent(20))
            .expect("should borrow");
        assert_eq!(total.total_principal_due(), Coin::new(10000));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(&env.block.time);
        assert_eq!(interest_due, Coin::new(1000));

        total.repay(
            env.block.time,
            Coin::new(1000),
            Coin::new(5000),
            Percent::from_percent(20),
        );
        assert_eq!(total.total_principal_due(), Coin::new(5000));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(&env.block.time);
        assert_eq!(interest_due, 500u128.into());
    }

    #[test]
    fn borrow_and_repay_overflow() {
        let mut deps = testing::mock_dependencies();
        let mut env = testing::mock_env();

        let total: Total<StableC> = Total::default();
        total.store(deps.as_mut().storage).expect("should store");

        let mut total: Total<StableC> = Total::load(deps.as_ref().storage).expect("should load");

        assert_eq!(total.total_principal_due(), Coin::<StableC>::new(0));

        total
            .borrow(env.block.time, Coin::new(9999), Percent::from_percent(20))
            .expect("should borrow");
        assert_eq!(total.total_principal_due(), Coin::new(9999));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 4);
        let interest_due = total.total_interest_due_by_now(&env.block.time);
        assert_eq!(interest_due, Coin::new(499));

        total
            .borrow(env.block.time, Coin::new(8900), Percent::from_percent(15))
            .expect("should borrow");
        assert_eq!(total.total_principal_due(), Coin::new(18899));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(&env.block.time);
        assert_eq!(interest_due, Coin::new(2166));

        total.repay(
            env.block.time,
            Coin::new(2166),
            Coin::new(5000),
            Percent::from_percent(20),
        );
        assert_eq!(total.total_principal_due(), Coin::new(13899));
    }
}
