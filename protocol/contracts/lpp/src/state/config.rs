use std::mem;

use serde::{Deserialize, Serialize};

use currency::{Currency, SymbolSlice};
use finance::{percent::bound::BoundToHundredPercent, price::Price};
use lpp_platform::NLpn;
use platform::contract::Code;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::{
    borrow::InterestRate,
    error::{ContractError, Result},
    msg::InstantiateMsg,
};

pub(crate) use migration::migrate;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lease_code: Code,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    pub fn try_new<Lpn>(msg: InstantiateMsg, lease_code: Code) -> Result<Self>
    where
        Lpn: ?Sized + Currency,
    {
        if msg.lpn_ticker == Self::lpn_ticker::<Lpn>() {
            Ok(Self {
                lease_code,
                borrow_rate: msg.borrow_rate,
                min_utilization: msg.min_utilization,
            })
        } else {
            Err(ContractError::InvalidConfigParameter(
                "The LPN ticker does not match the LPN this contract is compiled with".into(),
            ))
        }
    }

    #[cfg(test)]
    pub fn new_unchecked(
        lease_code: Code,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    ) -> Self {
        Self {
            lease_code,
            borrow_rate,
            min_utilization,
        }
    }

    pub fn lpn_ticker<Lpn>() -> &'static SymbolSlice
    where
        Lpn: ?Sized + Currency,
    {
        Lpn::TICKER
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub const fn borrow_rate(&self) -> &InterestRate {
        &self.borrow_rate
    }

    pub const fn min_utilization(&self) -> BoundToHundredPercent {
        self.min_utilization
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn initial_derivative_price<Lpn>() -> Price<NLpn, Lpn>
    where
        Lpn: 'static + ?Sized,
    {
        Price::identity()
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Code) -> Result<()> {
        Self::update_field(storage, |config| Self {
            lease_code,
            ..config
        })
    }

    pub fn update_borrow_rate(storage: &mut dyn Storage, borrow_rate: InterestRate) -> Result<()> {
        Self::update_field(storage, |config| Self {
            borrow_rate,
            ..config
        })
    }

    pub fn update_min_utilization(
        storage: &mut dyn Storage,
        min_utilization: BoundToHundredPercent,
    ) -> Result<()> {
        Self::update_field(storage, |config| Self {
            min_utilization,
            ..config
        })
    }

    fn update_field<F>(storage: &mut dyn Storage, f: F) -> Result<()>
    where
        F: FnOnce(Config) -> Config,
    {
        Self::STORAGE
            .update(storage, |config: Self| Ok(f(config)))
            .map(mem::drop)
    }
}

mod migration {
    use std::str;

    use currency::Currency;
    use serde::{Deserialize, Serialize, Serializer};

    use finance::percent::bound::BoundToHundredPercent;
    use platform::contract::Code;
    use sdk::{
        cosmwasm_std::{Storage, Uint64},
        cw_storage_plus::Item,
    };

    use crate::{
        borrow::InterestRate,
        error::{ContractError, Result as ContractResult},
    };

    use super::Config;

    #[derive(Deserialize)]
    struct OldConfig {
        lpn_ticker: String,
        lease_code_id: Uint64,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    }

    impl Serialize for OldConfig {
        fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            unimplemented!("Required by `cw_storage_plus::Item::load`'s trait bounds.")
        }
    }

    pub(crate) fn migrate<Lpn>(storage: &mut dyn Storage) -> ContractResult<()>
    where
        Lpn: ?Sized + Currency,
    {
        let old_store: Item<'static, OldConfig> =
            Item::new(str::from_utf8(Config::STORAGE.as_slice()).expect("valid storage key "));
        old_store
            .load(storage)
            .map_err(Into::into)
            .and_then(|old_cfg: OldConfig| {
                if old_cfg.lpn_ticker == Config::lpn_ticker::<Lpn>() {
                    Ok(Config {
                        lease_code: Code::unchecked(old_cfg.lease_code_id.into()),
                        borrow_rate: old_cfg.borrow_rate,
                        min_utilization: old_cfg.min_utilization,
                    })
                } else {
                    let err_msg = format!(
                        "The configured LPN \"{loaded_ticker}\" does not match \
                            the statically set one \"{static_ticker}\"",
                        loaded_ticker = old_cfg.lpn_ticker,
                        static_ticker = Config::lpn_ticker::<Lpn>(),
                    );
                    Err(ContractError::InvalidConfigParameter(err_msg))
                }
            })
            .and_then(|config: Config| config.store(storage))
    }
}
