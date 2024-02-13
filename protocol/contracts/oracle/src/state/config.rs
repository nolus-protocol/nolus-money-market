use currency::SymbolOwned;
use marketprice::config::Config as PriceConfig;
use sdk::{cosmwasm_ext::as_dyn::storage, cosmwasm_std::StdResult, cw_storage_plus::Item};

use crate::{api::Config, ContractError};

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn new(base_asset: SymbolOwned, price_config: PriceConfig) -> Self {
        Self {
            base_asset,
            price_config,
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

    pub fn update<S>(storage: &mut S, price_config: PriceConfig) -> Result<(), ContractError>
    where
        S: storage::DynMut + ?Sized,
    {
        Self::STORAGE
            .update(storage.as_dyn_mut(), |mut c| -> StdResult<_> {
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
