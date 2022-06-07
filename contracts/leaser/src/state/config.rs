use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use finance::percent::Percent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    msg::{InstantiateMsg, Liability, Repayment},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub lease_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: Percent,
    pub recalc_hours: u16,
    pub liability: Liability,
    pub repayment: Repayment,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(sender: Addr, msg: InstantiateMsg) -> Result<Self, ContractError> {
        Ok(Config {
            owner: sender,
            lease_code_id: msg.lease_code_id.u64(),
            lpp_ust_addr: msg.lpp_ust_addr,
            lease_interest_rate_margin: msg.lease_interest_rate_margin,
            recalc_hours: msg.recalc_hours,
            liability: msg.liability,
            repayment: msg.repayment,
        })
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        lease_interest_rate_margin: Percent,
        liability: crate::msg::Liability,
        repayment: Repayment,
    ) -> Result<(), ContractError> {
        Self::load(storage)?;
        Self::STORAGE.update(storage, |mut c| -> Result<Config, ContractError> {
            c.lease_interest_rate_margin = lease_interest_rate_margin;
            c.liability = liability;
            c.repayment = repayment;
            Ok(c)
        })?;
        Ok(())
    }
}
