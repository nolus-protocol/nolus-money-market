use std::mem;

use serde::{Deserialize, Serialize};

use finance::{duration::Duration, percent::Percent};
use lease::api::open::{ConnectionParams, PositionSpecDTO};
use platform::contract::Code;
use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{msg::InstantiateMsg, result::ContractResult};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
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
    pub dex: ConnectionParams,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

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
            dex: msg.dex,
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update(
        storage: &mut dyn Storage,
        lease_interest_rate_margin: Percent,
        lease_position_spec: PositionSpecDTO,
        lease_due_period: Duration,
    ) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |c| {
                ContractResult::Ok(Self {
                    lease_interest_rate_margin,
                    lease_position_spec,
                    lease_due_period,
                    ..c
                })
            })
            .map(mem::drop)
            .map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, new_code: Code) -> ContractResult<()> {
        Self::STORAGE
            .update(storage, |c| -> ContractResult<Config> {
                Ok(Self {
                    lease_code: new_code,
                    ..c
                })
            })
            .map(mem::drop)
            .map_err(Into::into)
    }
}

pub mod migrate {
    use cosmwasm_std::{Addr, Storage};
    use finance::{duration::Duration, percent::Percent};
    use lease::api::open::{ConnectionParams, PositionSpecDTO};
    use platform::contract::Code;
    use sdk::cw_storage_plus::Item;
    use serde::{Deserialize, Serialize, Serializer};

    use crate::result::ContractResult;

    use super::Config;

    const STORAGE_OLD: Item<'static, ConfigOld> = Item::new("config");

    #[derive(Deserialize)]
    pub struct ConfigOld {
        pub lease_code: Code,
        pub lpp: Addr,
        pub profit: Addr,
        pub reserve: Addr,
        pub time_alarms: Addr,
        pub market_price_oracle: Addr,
        pub lease_position_spec: PositionSpecDTO,
        pub lease_interest_rate_margin: Percent,
        pub lease_due_period: Duration,
        pub dex: ConnectionParams,
    }
    impl Serialize for ConfigOld {
        fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            unimplemented!("Required by `cw_storage_plus::Item::load`'s trait bounds.")
        }
    }

    pub fn migrate(storage: &mut dyn Storage, protocols_registry: Addr) -> ContractResult<()> {
        STORAGE_OLD
            .load(storage)
            .map_err(Into::into)
            .map(|old_cfg: ConfigOld| Config {
                lease_code: old_cfg.lease_code,
                lpp: old_cfg.lpp,
                profit: old_cfg.profit,
                reserve: old_cfg.reserve,
                time_alarms: old_cfg.time_alarms,
                market_price_oracle: old_cfg.market_price_oracle,
                protocols_registry,
                lease_position_spec: old_cfg.lease_position_spec,
                lease_interest_rate_margin: old_cfg.lease_interest_rate_margin,
                lease_due_period: old_cfg.lease_due_period,
                dex: old_cfg.dex,
            })
            .and_then(|config: Config| config.store(storage))
    }
}
