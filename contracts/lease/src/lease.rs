use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::{
    application::{Denom, Liability},
    loan::{Loan},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    customer: Addr,
    currency: Denom,
    liability: Liability,
    interest: Loan,
}

const DB_ITEM: Item<Lease> = Item::new("lease");

impl Lease {
    pub fn new(
        customer: Addr,
        currency: Denom,
        liability: Liability,
        interest: Loan,
    ) -> Self {
        Self {
            customer,
            currency,
            liability,
            interest,
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

    use crate::{application::Liability, loan::Loan};

    use super::Lease;

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = Lease {
            customer: Addr::unchecked("test"),
            currency: "UST".to_owned(),
            liability: Liability {
                init_percent: 24,
                healthy_percent: 32,
                max_percent: 40,
                recalc_secs: 3,
            },
            interest: Loan::new(23, Addr::unchecked("ust_lpp"), 100, 10),
        };
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded = Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp, obj_loaded);
    }
}
