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

        let new_total_principal_due = self
            .total_principal_due
            .checked_add(amount)
            .ok_or(ContractError::OverflowError("Total principal due overflow"))?;

        // TODO: get rid of fully qualified syntax
        let interest_sum =
            Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                .checked_add(loan_interest_rate.of(amount))
                .ok_or(ContractError::OverflowError(
                    "Annual interest rate calculation overflow",
                ))?;

        self.annual_interest_rate = Rational::new(interest_sum, new_total_principal_due);

        self.total_principal_due = new_total_principal_due;

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
        // Due to rounding in the calculations, there is a possibility of overflow in the arithmetic operations.
        // A saturating sub method will ensure that even if this happens, the result of the subtraction will become 0.
        self.total_interest_due = self
            .total_interest_due_by_now(&ctime)
            .saturating_sub(loan_interest_payment);

        self.annual_interest_rate = if self.total_principal_due > loan_principal_payment {
            Rational::new(
                Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
                    - loan_interest_rate.of(loan_principal_payment),
                self.total_principal_due - loan_principal_payment,
            )
        } else {
            zero_interest_rate()
        };

        self.total_principal_due = self
            .total_principal_due
            .saturating_sub(loan_principal_payment);

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

    use crate::loan::Loan;

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
    fn borrow_and_repay_with_overflow() {
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_nanos(0);

        let mut total: Total<LpnC> = Total::default();
        assert_eq!(total.total_principal_due(), Coin::<LpnC>::new(0));

        let borrow_loan1 = Coin::<LpnC>::new(5_458_329);
        let loan1_annual_interest_rate = Percent::from_permille(137);
        let loan1 = Loan {
            principal_due: borrow_loan1,
            annual_interest_rate: loan1_annual_interest_rate,
            interest_paid: env.block.time,
        };

        total
            .borrow(env.block.time, borrow_loan1, loan1_annual_interest_rate)
            .unwrap();
        assert_eq!(total.total_principal_due(), borrow_loan1);
        assert_eq!(total.total_interest_due_by_now(&env.block.time), Coin::ZERO);

        env.block.time = env.block.time.plus_days(59);

        // Open loan2 after 59 days
        let borrow_loan2 = Coin::<LpnC>::new(3_543_118);
        let loan2_annual_interest_rate = Percent::from_permille(133);
        let loan2 = Loan {
            principal_due: borrow_loan2,
            annual_interest_rate: loan2_annual_interest_rate,
            interest_paid: env.block.time,
        };

        let total_interest_due = total.total_interest_due_by_now(&env.block.time);
        assert_eq!(total_interest_due, loan1.interest_due(&env.block.time));

        total
            .borrow(env.block.time, borrow_loan2, loan2_annual_interest_rate)
            .unwrap();
        assert_eq!(total.total_principal_due(), borrow_loan1 + borrow_loan2);
        assert_eq!(
            total.total_interest_due_by_now(&env.block.time),
            total_interest_due
        );

        env.block.time = env.block.time.plus_days(147);

        // Fully repay loan1 after 147 days
        total.repay(
            env.block.time,
            loan1.interest_due(&env.block.time),
            loan1.principal_due,
            loan1.annual_interest_rate,
        );
        assert_eq!(total.total_principal_due(), borrow_loan2);

        env.block.time = env.block.time.plus_days(67);

        // Fully repay loan2 after 67 days
        total.repay(
            env.block.time,
            loan2.interest_due(&env.block.time),
            loan2.principal_due,
            loan2.annual_interest_rate,
        );

        assert!(total.total_interest_due.is_zero());
        assert!(total.total_principal_due.is_zero());
    }
}
