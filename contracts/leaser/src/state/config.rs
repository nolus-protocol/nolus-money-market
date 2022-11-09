use lease::api::dex::ConnectionParams;
use serde::{Deserialize, Serialize};

use finance::{liability::Liability, percent::Percent};
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{
    error::ContractResult,
    msg::{InstantiateMsg, Repayment},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub lease_code_id: u64,
    pub lpp_addr: Addr,
    pub lease_interest_rate_margin: Percent,
    pub liability: Liability,
    pub repayment: Repayment,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub profit: Addr,
    pub dex: Option<ConnectionParams>,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(sender: Addr, msg: InstantiateMsg) -> Result<Self, ContractError> {
        Ok(Config {
            owner: sender,
            lease_code_id: msg.lease_code_id.u64(),
            lpp_addr: msg.lpp_ust_addr,
            lease_interest_rate_margin: msg.lease_interest_rate_margin,
            liability: msg.liability,
            repayment: msg.repayment,
            time_alarms: msg.time_alarms,
            market_price_oracle: msg.market_price_oracle,
            profit: msg.profit,
            dex: None,
        })
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn setup_dex(storage: &mut dyn Storage, params: ConnectionParams) -> ContractResult<()> {
        Self::STORAGE.update(storage, |mut c| {
            if c.dex.is_none() {
                c.dex = Some(params);
                Ok(c)
            } else {
                Err(ContractError::DEXConnectivityAlreadySetup {})
            }
        })?;
        Ok(())
    }

    pub fn update(
        storage: &mut dyn Storage,
        lease_interest_rate_margin: Percent,
        liability: Liability,
        repayment: Repayment,
    ) -> Result<(), ContractError> {
        Self::load(storage)?;
        Self::STORAGE.update(storage, |mut c| {
            c.lease_interest_rate_margin = lease_interest_rate_margin;
            c.liability = liability;
            c.repayment = repayment;
            ContractResult::Ok(c)
        })?;
        Ok(())
    }
}
