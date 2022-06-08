use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint128, Timestamp, Storage, StdResult};
use cw_storage_plus::Item;
use finance::duration::Duration;
use finance::percent::Percent;
use crate::error::ContractError;
use finance::interest::InterestPeriod;

// TODO: evaluate fixed or rust_decimal instead of cosmwasm_std::Decimal
// https://docs.rs/fixed/latest/fixed/index.html
// https://docs.rs/rust_decimal/latest/rust_decimal/index.html
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Total {
    total_principal_due: Uint128,
    total_interest_due: Uint128,
    annual_interest_rate: Percent,
    last_update_time: Timestamp,
}


impl Total {
    const STORAGE: Item<'static, Total> = Item::new("total");

    pub fn total_principal_due(&self) -> Uint128 { self.total_principal_due }
    pub fn total_interest_due(&self) -> Uint128 { self.total_interest_due }
    pub fn annual_interest_rate(&self) -> Percent { self.annual_interest_rate }
    pub fn last_update_time(&self) -> Timestamp { self.last_update_time }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn borrow(&mut self, ctime: Timestamp, amount: Uint128, loan_interest_rate: Percent) -> Result<&Self, ContractError> {

        self.total_interest_due = self.total_interest_due_by_now(ctime);


        let annual_interest_rate_permilles =
            (self.annual_interest_rate.of(self.total_principal_due)
                + loan_interest_rate.of(amount)).u128()*1000/
            (self.total_principal_due + amount).u128()
        ;

        self.annual_interest_rate = Percent::from_permille(annual_interest_rate_permilles.try_into()?);

        self.total_principal_due += amount;

        self.last_update_time = ctime;

        Ok(self)
    }

    pub fn repay(&mut self, ctime: Timestamp, loan_principal_payment: Uint128, loan_interest_rate: Percent) -> Result<&Self, ContractError> {

        self.total_interest_due = self.total_interest_due_by_now(ctime);

        self.annual_interest_rate = if self.total_principal_due == loan_principal_payment {
            Percent::ZERO
        } else {
                let permilles = (
                    self.annual_interest_rate.of(self.total_principal_due)
                        - loan_interest_rate.of(loan_principal_payment)
                ).u128()*1000/
                (self.total_principal_due - loan_principal_payment).u128();
                Percent::from_permille(permilles.try_into()?)
        };

        self.total_principal_due -= loan_principal_payment;

        self.last_update_time = ctime;

        Ok(self)
    }

    pub fn total_interest_due_by_now(&self, ctime: Timestamp) -> Uint128 {
        InterestPeriod::with_interest(self.annual_interest_rate)
            .from(self.last_update_time)
            .spanning(Duration::between(self.last_update_time, ctime))
            .interest(self.total_principal_due)
            + self.total_interest_due
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing;
    
    #[test]
    fn borrow_and_repay() {
        let mut deps = testing::mock_dependencies();
        let env = testing::mock_env();

        let mut total = Total::default();
        total.store(deps.as_mut().storage)
            .expect("should store");

        total.borrow(env.block.time, 10000u128.into(), Percent::from_percent(10))
            .expect("should borrow");
   }
    
}

