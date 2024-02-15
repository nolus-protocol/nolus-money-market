use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::as_dyn::storage,
    cosmwasm_std::{Addr, StdResult},
    cw_storage_plus::Item,
};

use crate::{error::ContractError, result::ContractResult};

use super::reward_scale::RewardScale;

pub type CadenceHours = u16;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    // Time duration in hours defining the periods of time this instance is awaken
    pub cadence_hours: CadenceHours,
    // Protocols registry
    pub protocols_registry: Addr,
    // address to treasury contract
    pub treasury: Addr,
    // A list of (minTVL_MNLS: u32, APR%o) which defines the APR as per the TVL.
    pub tvl_to_apr: RewardScale,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("dispatcher_config");

    pub fn new(
        cadence_hours: CadenceHours,
        protocols_registry: Addr,
        treasury: Addr,
        tvl_to_apr: RewardScale,
    ) -> Self {
        Config {
            cadence_hours,
            protocols_registry,
            tvl_to_apr,
            treasury,
        }
    }

    pub fn store<S>(self, storage: &mut S) -> StdResult<()>
    where
        S: storage::DynMut + ?Sized,
    {
        Self::STORAGE.save(storage.as_dyn_mut(), &self)
    }

    pub fn load<S>(storage: &S) -> StdResult<Self>
    where
        S: storage::Dyn + ?Sized,
    {
        Self::STORAGE.load(storage.as_dyn())
    }

    pub fn update_cadence_hours<S>(
        storage: &mut S,
        cadence_hours: CadenceHours,
    ) -> ContractResult<()>
    where
        S: storage::DynMut + ?Sized,
    {
        Self::STORAGE
            .update(
                storage.as_dyn_mut(),
                |config| -> Result<Config, ContractError> {
                    Ok(Self {
                        cadence_hours,
                        ..config
                    })
                },
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn update_tvl_to_apr<S>(storage: &mut S, tvl_to_apr: RewardScale) -> ContractResult<()>
    where
        S: storage::DynMut + ?Sized,
    {
        Self::STORAGE
            .update(
                storage.as_dyn_mut(),
                |config| -> Result<Config, ContractError> {
                    Ok(Self {
                        tvl_to_apr,
                        ..config
                    })
                },
            )
            .map(|_| ())
            .map_err(Into::into)
    }
}

pub(crate) mod migration {
    use serde::{Deserialize, Serialize, Serializer};

    use sdk::cosmwasm_ext::as_dyn::{storage, AsDyn};
    use sdk::{cosmwasm_std::Addr, cw_storage_plus::Item};

    use crate::{
        result::ContractResult,
        state::{reward_scale::RewardScale, CadenceHours, Config},
    };

    const STORAGE: Item<'static, OldConfig> = Item::new("dispatcher_config");

    #[derive(Deserialize)]
    struct OldConfig {
        cadence_hours: CadenceHours,
        treasury: Addr,
        tvl_to_apr: RewardScale,
    }

    impl Serialize for OldConfig {
        fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            unimplemented!("Required by `cw_storage_plus::Item::load`'s trait bounds.")
        }
    }

    impl OldConfig {
        fn migrate(self, protocols_registry: Addr) -> Config {
            Config::new(
                self.cadence_hours,
                protocols_registry,
                self.treasury,
                self.tvl_to_apr,
            )
        }
    }

    pub fn migrate<S>(storage: &mut S, protocols_registry: Addr) -> ContractResult<()>
    where
        S: storage::DynMut + ?Sized,
    {
        STORAGE
            .load(storage.as_dyn())
            .map(|old| old.migrate(protocols_registry))
            .and_then(|config: Config| config.store(storage))
            .map_err(Into::into)
    }
}
