use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use lpp::stub::Lpp;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{liability::Liability, loan::Loan, opening::Denom};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease<L> {
    customer: Addr,
    currency: Denom,
    liability: Liability,
    loan: Loan<L>,
}

impl<L> Lease<L>
where
    L: Lpp + Serialize + DeserializeOwned,
{
    const DB_ITEM: Item<'static, Lease<L>> = Item::new("lease");

    pub fn new(customer: Addr, currency: Denom, liability: Liability, loan: Loan<L>) -> Self {
        Self {
            customer,
            currency,
            liability,
            loan,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Lease::DB_ITEM.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Lease::DB_ITEM.load(storage)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Addr, Coin};
    use lpp::stub::Lpp;
    use serde::{Deserialize, Serialize};

    use crate::{liability::Liability, loan::Loan};

    use super::Lease;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {}
    impl Lpp for LppLocalStub {
        fn open_loan_async(&mut self, _amount: cosmwasm_std::Coin) -> cosmwasm_std::StdResult<()> {
            Ok(())
        }
    }

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = Lease {
            customer: Addr::unchecked("test"),
            currency: "UST".to_owned(),
            liability: Liability::new(65, 5, 10, 10 * 24),
            loan: Loan::open(Coin::new(23456, "UST"), LppLocalStub {}, 23, 100, 10).unwrap(),
        };
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded = Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp, obj_loaded);
    }
}
