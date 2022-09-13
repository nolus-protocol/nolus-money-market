use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use finance::{currency::SymbolOwned, liability::Liability};

use crate::loan::LoanDTO;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDTO {
    pub(crate) lease: Addr,
    pub(crate) customer: Addr,
    pub(crate) currency: SymbolOwned,
    pub(crate) liability: Liability,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: Addr,
    pub(crate) oracle: Addr,
}

impl<'a> LeaseDTO {
    const DB_ITEM: Item<'a, LeaseDTO> = Item::new("lease");

    pub(crate) fn new(
        lease: Addr,
        customer: Addr,
        currency: SymbolOwned,
        liability: Liability,
        loan: LoanDTO,
        time_alarms: Addr,
        oracle: Addr,
    ) -> Self {
        Self {
            lease,
            customer,
            currency,
            liability,
            loan,
            time_alarms,
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
