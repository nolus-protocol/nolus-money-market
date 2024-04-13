use std::mem;

use serde::{Deserialize, Serialize};

use finance::{duration::Duration, percent::Percent};
use lease::api::open::{ConnectionParams, PositionSpecDTO};
use platform::contract::Code;
use sdk::{
    cosmwasm_std::{Addr, QuerierWrapper, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

use crate::{msg::InstantiateMsg, result::ContractResult};
pub(crate) use migration::migrate;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct Config {
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

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn try_new(msg: InstantiateMsg, querier: &QuerierWrapper<'_>) -> ContractResult<Self> {
        Code::try_new(msg.lease_code.into(), querier)
            .map_err(Into::into)
            .map(|lease_code| Self {
                lease_code,
                lpp: msg.lpp,
                profit: msg.profit,
                reserve: msg.reserve,
                time_alarms: msg.time_alarms,
                market_price_oracle: msg.market_price_oracle,
                lease_position_spec: msg.lease_position_spec,
                lease_interest_rate_margin: msg.lease_interest_rate_margin,
                lease_due_period: msg.lease_due_period,
                dex: msg.dex,
            })
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

mod migration {
    use std::str;

    use serde::{Deserialize, Serialize, Serializer};

    use finance::{duration::Duration, percent::Percent};
    use lease::api::open::{ConnectionParams, PositionSpecDTO};
    use platform::contract::{Code, CodeId};
    use sdk::{
        cosmwasm_std::{Addr, Storage},
        cw_storage_plus::Item,
    };

    use crate::result::ContractResult;

    use super::Config;

    #[derive(Deserialize)]
    struct OldConfig {
        lease_code_id: CodeId,
        lpp: Addr,
        profit: Addr,
        time_alarms: Addr,
        market_price_oracle: Addr,
        lease_position_spec: PositionSpecDTO,
        lease_interest_rate_margin: Percent,
        lease_due_period: Duration,
        dex: ConnectionParams,
    }

    impl Serialize for OldConfig {
        fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            unimplemented!("Required by `cw_storage_plus::Item::load`'s trait bounds.")
        }
    }

    pub(crate) fn migrate(storage: &mut dyn Storage, reserve: Addr) -> ContractResult<()> {
        let old_store: Item<'_, OldConfig> =
            Item::new(str::from_utf8(Config::STORAGE.as_slice()).expect("valid storage key "));
        old_store
            .load(storage)
            .map_err(Into::into)
            .map(|old_cfg: OldConfig| Config {
                lease_code: Code::unchecked(old_cfg.lease_code_id),
                lpp: old_cfg.lpp,
                profit: old_cfg.profit,
                reserve,
                time_alarms: old_cfg.time_alarms,
                market_price_oracle: old_cfg.market_price_oracle,
                lease_position_spec: old_cfg.lease_position_spec,
                lease_interest_rate_margin: old_cfg.lease_interest_rate_margin,
                lease_due_period: old_cfg.lease_due_period,
                dex: old_cfg.dex,
            })
            .and_then(|config: Config| config.store(storage))
    }
}
