use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cosmwasm_std::{Uint128, Decimal, Timestamp, Storage, StdResult, Env};
use cw_storage_plus::Item;
use crate::calc;
use std::ops::Deref;

#[derive(Clone, Debug, Default)]
pub struct Total {
    data: TotalData,
}

// TODO: evaluate fixed or rust_decimal instead of cosmwasm_std::Decimal
// https://docs.rs/fixed/latest/fixed/index.html
// https://docs.rs/rust_decimal/latest/rust_decimal/index.html
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct TotalData {
    pub total_principal_due: Uint128,
    pub total_interest_due: Uint128,
    pub annual_interest_rate: Decimal,
    pub last_update_time: Timestamp,
}

impl Deref for Total {
    type Target = TotalData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Total {
    const STORAGE: Item<'static, TotalData> = Item::new("total");

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self.data)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        let data = Self::STORAGE.load(storage)?;
        Ok(Self { data })
    }

    pub fn borrow(&mut self, env: &Env, amount: Uint128, loan_interest_rate: Decimal) -> &Self {

        self.data.total_interest_due = self.total_interest_due_by_now(env);

        let total = &mut self.data;

        total.annual_interest_rate = Decimal::from_ratio(
            total.annual_interest_rate * total.total_principal_due
                + loan_interest_rate * amount,
            total.total_principal_due + amount
        );

        total.total_principal_due += amount;

        total.last_update_time = env.block.time;

        self
    }

    pub fn repay(&mut self, env: &Env, loan_principal_payment: Uint128, loan_interest_rate: Decimal) -> &Self {

        self.data.total_interest_due = self.total_interest_due_by_now(env);

        let total = &mut self.data;

        total.annual_interest_rate = if total.total_principal_due == loan_principal_payment {
            Decimal::zero()
        } else {
            Decimal::from_ratio(
                total.annual_interest_rate * total.total_principal_due
                    - loan_interest_rate * loan_principal_payment,
                total.total_principal_due - loan_principal_payment,
            )
        };

        total.total_principal_due -= loan_principal_payment;

        total.last_update_time = env.block.time;

        self
    }


}

impl TotalData {
    pub fn total_interest_due_by_now(&self, env: &Env) -> Uint128 {
        self.total_interest_due + calc::interest(
            self.total_principal_due,
            self.annual_interest_rate,
            calc::dt(env, self.last_update_time))
    }
}



