use std::mem;

use serde::{Deserialize, Serialize};

use dex::ConnectionParams;
use finance::{duration::Duration, percent::Percent};
use lease::api::{limits::MaxSlippage, open::PositionSpecDTO};
use platform::contract::Code;
use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};

use crate::{
    msg::{InstantiateMsg, NewConfig},
    result::ContractResult,
};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Clone, Debug))]
pub struct Config {
    pub lease_code: Code,
    pub lpp: Addr,
    pub profit: Addr,
    pub reserve: Addr,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub protocols_registry: Addr,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_interest_rate_margin: Percent,
    pub lease_due_period: Duration,
    pub lease_max_slippage: MaxSlippage,
    pub lease_admin: Addr,
    pub dex: ConnectionParams,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(lease_code: Code, msg: InstantiateMsg) -> Self {
        Self {
            lease_code,
            lpp: msg.lpp,
            profit: msg.profit,
            reserve: msg.reserve,
            time_alarms: msg.time_alarms,
            market_price_oracle: msg.market_price_oracle,
            protocols_registry: msg.protocols_registry,
            lease_position_spec: msg.lease_position_spec,
            lease_interest_rate_margin: msg.lease_interest_rate_margin,
            lease_due_period: msg.lease_due_period,
            lease_max_slippage: msg.lease_max_slippage,
            lease_admin: msg.lease_admin,
            dex: msg.dex,
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update(storage: &mut dyn Storage, new_config: NewConfig) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |c| {
                Ok(Self {
                    lease_interest_rate_margin: new_config.lease_interest_rate_margin,
                    lease_position_spec: new_config.lease_position_spec,
                    lease_due_period: new_config.lease_due_period,
                    lease_max_slippage: new_config.lease_max_slippage,
                    ..c
                })
            })
            .map(mem::drop)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, new_code: Code) -> ContractResult<Self> {
        Self::STORAGE.update(storage, |c| -> ContractResult<Config> {
            Ok(Self {
                lease_code: new_code,
                ..c
            })
        })
    }
}
