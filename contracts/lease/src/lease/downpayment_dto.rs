use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, CoinDTO},
    currency::SymbolOwned,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownpaymentDTO {
    pub(super) downpayment: CoinDTO,
}

impl<'a> DownpaymentDTO {
    const DB_ITEM: Item<'a, DownpaymentDTO> = Item::new("downpayment");

    pub(crate) fn new(downpayment: CoinDTO) -> Self {
        Self { downpayment }
    }

    pub(crate) fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, self)
    }

    pub(crate) fn remove(storage: &mut dyn Storage) -> StdResult<Self> {
        let item = Self::DB_ITEM.load(storage)?;

        Self::DB_ITEM.remove(storage);

        Ok(item)
    }

    pub(crate) const fn amount(&self) -> Amount {
        self.downpayment.amount()
    }

    pub(crate) const fn symbol(&self) -> &SymbolOwned {
        self.downpayment.symbol()
    }
}
