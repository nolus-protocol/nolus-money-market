use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO define it as type not alias
pub type Denom = String;

#[derive(Clone, Debug, PartialEq, JsonSchema, Serialize, Deserialize)]
pub struct Application {
    /// The customer who has opened the lease.
    customer: Addr,
    /// Denomination of the currency this lease is about.
    currency: Denom,
    /// The delta, represented as permille, added on top of the LPP Loan interest rate.
    ///
    /// The value remain intact. The amount, a part of any payment, goes to the Profit contract.
    annual_margin_interest_permille: u64,
}

const DB_ITEM: Item<Application> = Item::new("application");

impl Application {
    pub fn new(customer: Addr, currency: Denom, annual_margin_interest_permille: u64) -> Self {
        Self {
            customer,
            currency,
            annual_margin_interest_permille,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        DB_ITEM.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        DB_ITEM.load(storage)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Addr};

    use super::Application;

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = Application::new(Addr::unchecked("test"), "UST".to_owned(), 750);
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded = Application::load(&storage).expect("loading failed");
        assert_eq!(obj_exp, obj_loaded);
    }
}
