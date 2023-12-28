use currency::SymbolOwned;
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::{api::Config, ContractError};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(base_asset: SymbolOwned, price_config: PriceConfig) -> Self {
        Self {
            base_asset,
            price_config,
        }
    }

    pub fn store(self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, &self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update(
        storage: &mut dyn Storage,
        price_config: PriceConfig,
    ) -> Result<(), ContractError> {
        Self::STORAGE
            .update(storage, |mut c| -> StdResult<_> {
                c.price_config = price_config;
                Ok(c)
            })
            .map(|_| ())
            .map_err(ContractError::UpdateConfig)
    }
}

#[cfg(test)]
mod test {
    use currency::SymbolOwned;
    use finance::{duration::Duration, percent::Percent};
    use marketprice::config::Config as PriceConfig;
    use oracle_platform::msg::Config as PlatformConfig;
    use sdk::cosmwasm_std::{from_json, to_json_vec};

    use super::Config;

    #[test]
    fn impl_config() {
        let base_asset: SymbolOwned = "base_asset".into();
        let cfg = Config::new(
            base_asset.clone(),
            PriceConfig::new(
                Percent::from_percent(35),
                Duration::from_secs(10),
                12,
                Percent::from_percent(70),
            ),
        );
        let cfg_platform = from_json::<PlatformConfig>(&to_json_vec(&cfg).unwrap()).unwrap();
        assert_eq!(cfg_platform.base_asset, base_asset);
    }
}
