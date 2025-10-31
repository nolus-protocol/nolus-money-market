use std::mem;

use serde::{Deserialize, Serialize};

use dex::ConnectionParams;
use finance::{duration::Duration, percent::Percent100};
use lease::api::{limits::MaxSlippages, open::PositionSpecDTO};
use platform::contract::Code;
use sdk::{
    cosmwasm_std::{Addr, StdError as SdkError, Storage},
    cw_storage_plus::Item,
};

use crate::{
    ContractError,
    msg::{InstantiateMsg, NewConfig},
    result::ContractResult,
};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
pub struct Config {
    pub lease_code: Code,
    pub lpp: Addr,
    pub profit: Addr,
    pub reserve: Addr,
    pub time_alarms: Addr,
    pub market_price_oracle: Addr,
    pub protocols_registry: Addr,
    pub lease_position_spec: PositionSpecDTO,
    pub lease_interest_rate_margin: Percent100,
    pub lease_due_period: Duration,
    pub lease_max_slippages: MaxSlippages,
    pub lease_admin: Addr,
    pub dex: ConnectionParams,
    pub contract_owner: Addr,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(
        lease_code: Code,
        InstantiateMsg {
            lease_code: _,
            lpp,
            profit,
            reserve,
            time_alarms,
            market_price_oracle,
            protocols_registry,
            lease_position_spec,
            lease_interest_rate_margin,
            lease_due_period,
            lease_max_slippages,
            lease_admin,
            dex,
        }: InstantiateMsg,
        contract_owner: Addr,
    ) -> Self {
        Self {
            lease_code,
            lpp,
            profit,
            reserve,
            time_alarms,
            market_price_oracle,
            protocols_registry,
            lease_position_spec,
            lease_interest_rate_margin,
            lease_due_period,
            lease_max_slippages,
            lease_admin,
            dex,
            contract_owner,
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE
            .save(storage, self)
            .map_err(ContractError::SaveConfigFailure)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE
            .load(storage)
            .map_err(ContractError::LoadConfigFailure)
    }

    pub fn update(storage: &mut dyn Storage, new_config: NewConfig) -> ContractResult<()> {
        Self::STORAGE
            .update::<_, UpdateDataError>(storage, |c| {
                Ok(Self {
                    lease_interest_rate_margin: new_config.lease_interest_rate_margin,
                    lease_position_spec: new_config.lease_position_spec,
                    lease_due_period: new_config.lease_due_period,
                    lease_max_slippages: new_config.lease_max_slippages,
                    ..c
                })
            })
            .map_err(Into::into)
            .map(mem::drop)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, new_code: Code) -> ContractResult<Self> {
        Self::STORAGE
            .update::<_, UpdateDataError>(storage, |c| {
                Ok(Self {
                    lease_code: new_code,
                    ..c
                })
            })
            .map_err(Into::into)
    }

    pub fn update_lease_admin(storage: &mut dyn Storage, new_admin: Addr) -> ContractResult<Self> {
        Self::STORAGE
            .update::<_, UpdateDataError>(storage, |c| {
                Ok(Self {
                    lease_admin: new_admin,
                    ..c
                })
            })
            .map_err(Into::into)
    }

    pub const fn contract_owner(&self) -> &Addr {
        &self.contract_owner
    }
}

struct UpdateDataError(SdkError);
impl From<SdkError> for UpdateDataError {
    fn from(value: SdkError) -> Self {
        Self(value)
    }
}
impl From<UpdateDataError> for ContractError {
    fn from(value: UpdateDataError) -> Self {
        Self::UpdateConfigFailure(value.0)
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod tests {
    mod update_lease_admin {
        use sdk::cosmwasm_std::{Addr, testing::MockStorage};

        use crate::{ContractError, msg::Config, tests};

        #[test]
        fn no_config() {
            let mut storage = MockStorage::new();
            let new_admin = Addr::unchecked("my successor");
            assert!(matches!(
                Config::update_lease_admin(&mut storage, new_admin).unwrap_err(),
                ContractError::UpdateConfigFailure(_)
            ));
        }

        #[test]
        fn has_config() {
            let mut storage = MockStorage::new();
            let mut config = tests::config();
            config.store(&mut storage).unwrap();

            let new_admin = Addr::unchecked("my successor");
            let config_updated =
                Config::update_lease_admin(&mut storage, new_admin.clone()).unwrap();
            config.lease_admin = new_admin;
            assert_eq!(config, config_updated);
            assert_eq!(config, Config::load(&storage).unwrap());
        }
    }
}
