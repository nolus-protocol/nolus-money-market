use crate::error::ContractError;
use cosmwasm_std::{StdResult, Storage, Timestamp};
use cw_storage_plus::Item;
use finance::coin::Coin;
use finance::currency::Currency;
use finance::duration::Duration;
use finance::fraction::Fraction;
use finance::interest::InterestPeriod;
use finance::percent::Percent;
use finance::ratio::Rational;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// TODO: evaluate fixed or rust_decimal instead of cosmwasm_std::Decimal
// https://docs.rs/fixed/latest/fixed/index.html
// https://docs.rs/rust_decimal/latest/rust_decimal/index.html
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct Total<LPN>
where
    LPN: Currency,
{
    total_principal_due: Coin<LPN>,
    total_interest_due: Coin<LPN>,
    annual_interest_rate: Rational<Coin<LPN>>,
    last_update_time: Timestamp,
}

impl<LPN> Default for Total<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<LPN> Total<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
{
    const STORAGE: Item<'static, Total<LPN>> = Item::new("total");

    pub fn new() -> Self {
        Total {
            total_principal_due: Coin::new(0),
            total_interest_due: Coin::new(0),
            annual_interest_rate: Rational::new(Coin::new(0), Coin::new(1000)),
            last_update_time: Timestamp::default(),
        }
    }

    pub fn total_principal_due(&self) -> Coin<LPN> {
        self.total_principal_due
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn total_interest_due_by_now(&self, ctime: Timestamp) -> Coin<LPN> {
        InterestPeriod::<Coin<LPN>, _>::with_interest(self.annual_interest_rate)
            .from(self.last_update_time)
            .spanning(Duration::between(self.last_update_time, ctime))
            .interest(self.total_principal_due)
            + self.total_interest_due
    }

    pub fn borrow(
        &mut self,
        ctime: Timestamp,
        amount: Coin<LPN>,
        loan_interest_rate: Percent,
    ) -> Result<&Self, ContractError> {
        self.total_interest_due = self.total_interest_due_by_now(ctime);

        // TODO: get ride of fully qualified syntax
        self.annual_interest_rate = Rational::new(
            <Rational<Coin<LPN>> as Fraction<Coin<LPN>>>::of(
                &self.annual_interest_rate,
                self.total_principal_due,
            ) + loan_interest_rate.of(amount),
            self.total_principal_due + amount,
        );

        self.total_principal_due += amount;

        self.last_update_time = ctime;

        Ok(self)
    }

    pub fn repay(
        &mut self,
        ctime: Timestamp,
        loan_principal_payment: Coin<LPN>,
        loan_interest_rate: Percent,
    ) -> Result<&Self, ContractError> {
        self.total_interest_due = self.total_interest_due_by_now(ctime);

        self.annual_interest_rate = if self.total_principal_due == loan_principal_payment {
            Rational::new(Coin::<LPN>::new(0), Coin::<LPN>::new(100))
        } else {
            Rational::new(
                <Rational<Coin<LPN>> as Fraction<Coin<LPN>>>::of(
                    &self.annual_interest_rate,
                    self.total_principal_due,
                ) - loan_interest_rate.of(loan_principal_payment),
                self.total_principal_due - loan_principal_payment,
            )
        };

        // TODO: maybe add -= for Coin?
        self.total_principal_due -= loan_principal_payment;

        self.last_update_time = ctime;

        Ok(self)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use cosmwasm_std::testing;
    use finance::currency::Usdc;
    use finance::duration::Duration;

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

        total
            .repay(env.block.time, Coin::new(5000), Percent::from_percent(20))
            .expect("should repay");
        assert_eq!(total.total_principal_due(), Coin::new(5000));

        env.block.time = Timestamp::from_nanos(env.block.time.nanos() + Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(env.block.time);
        assert_eq!(interest_due, 1500u128.into());
    }
}
