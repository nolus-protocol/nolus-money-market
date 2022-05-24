use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint128, Decimal, Timestamp, Storage, StdResult, Env};
use cw_storage_plus::Item;
use crate::calc;

// TODO: evaluate fixed or rust_decimal instead of cosmwasm_std::Decimal
// https://docs.rs/fixed/latest/fixed/index.html
// https://docs.rs/rust_decimal/latest/rust_decimal/index.html
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Total {
    pub total_principal_due: Uint128,
    pub total_interest_due: Uint128,
    pub annual_interest_rate: Decimal,
    pub last_update_time: Timestamp,
}

impl Total {
    const STORAGE: Item<'static, Self> = Item::new("total");

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn borrow(&mut self, env: &Env, amount: Uint128, loan_interest_rate: Decimal) -> &Self {

        self.total_interest_due = self.total_interest_due_by_now(env);

        self.annual_interest_rate = Decimal::from_ratio(
            self.annual_interest_rate * self.total_principal_due
                + loan_interest_rate * amount,
            self.total_principal_due + amount
        );

        self.total_principal_due += amount;

        self.last_update_time = env.block.time;

        self
    }

    pub fn repay(&mut self, env: &Env, loan_principal_payment: Uint128, loan_interest_rate: Decimal) -> &Self {

        self.total_interest_due = self.total_interest_due_by_now(env);
        self.annual_interest_rate = if self.total_principal_due == loan_principal_payment {
            Decimal::zero()
        } else {
            Decimal::from_ratio(
                self.annual_interest_rate * self.total_principal_due
                    - loan_interest_rate * loan_principal_payment,
                self.total_principal_due - loan_principal_payment,
            )
        };

        self.total_principal_due -= loan_principal_payment;

        self.last_update_time = env.block.time;

        self
    }

    pub fn total_interest_due_by_now(&self, env: &Env) -> Uint128 {
        self.total_interest_due + calc::interest(
            self.total_principal_due,
            self.annual_interest_rate,
            calc::dt(env, self.last_update_time))
    }

}

