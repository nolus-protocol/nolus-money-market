use cosmwasm_std::{Addr, StdResult, Storage, Coin};
use cw_storage_plus::Item;
use finance::liability::Liability;
use lpp::stub::Lpp;
use serde::{Deserialize, Serialize};

use crate::{loan::Loan, msg::Denom, error::ContractResult};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease<L> {
    customer: Addr,
    currency: Denom,
    liability: Liability,
    loan: Loan<L>,
}

impl<L> Lease<L>
where
    L: Lpp,
{
    const DB_ITEM: Item<'static, Lease<L>> = Item::new("lease");

    pub(crate) fn new(customer: Addr, currency: Denom, liability: Liability, loan: Loan<L>) -> Self {
        Self {
            customer,
            currency,
            liability,
            loan,
        }
    }

    pub(crate) fn repay(&self, _downpayment: Coin) -> ContractResult<()> {
        todo!()
    }
    
    pub(crate) fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Lease::DB_ITEM.save(storage, &self)
    }

    pub(crate) fn load(storage: &dyn Storage) -> StdResult<Self> {
        Lease::DB_ITEM.load(storage)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Addr, StdResult, SubMsg};
    use finance::{liability::Liability, percent::Percent};
    use lpp::stub::Lpp;
    use serde::{Deserialize, Serialize};

    use crate::loan::Loan;

    use super::Lease;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStub {}
    impl Lpp for LppLocalStub {
        fn open_loan_req(&self, _amount: cosmwasm_std::Coin) -> StdResult<SubMsg> {
            unimplemented!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> Result<(), String> {
            unimplemented!()
        }
    }

    #[test]
    fn persist_ok() {
        let mut storage = MockStorage::default();
        let obj = Lease {
            customer: Addr::unchecked("test"),
            currency: "UST".to_owned(),
            liability: Liability::new(
                Percent::from(65),
                Percent::from(5),
                Percent::from(10),
                10 * 24,
            ),
            loan: Loan::open(LppLocalStub {}, 23, 100, 10).unwrap(),
        };
        let obj_exp = obj.clone();
        obj.store(&mut storage).expect("storing failed");
        let obj_loaded = Lease::load(&storage).expect("loading failed");
        assert_eq!(obj_exp, obj_loaded);
    }
}
