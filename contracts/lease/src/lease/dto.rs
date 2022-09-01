use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, liability::Liability};
use market_price_oracle::stub::OracleRef;

use crate::loan::LoanDTO;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDTO {
    pub(crate) customer: Addr,
    pub(crate) currency: SymbolOwned,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) oracle: OracleRef,
}

impl<'a> LeaseDTO {
    const DB_ITEM: Item<'a, LeaseDTO> = Item::new("lease");

    pub(crate) fn new(
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: LoanDTO,
        oracle: OracleRef,
    ) -> Self {
        Self {
            customer,
            currency,
            liability,
            loan,
            oracle,
        }
    }

    pub(crate) fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, self)
    }

    pub(crate) fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::DB_ITEM.load(storage)
    }
}
