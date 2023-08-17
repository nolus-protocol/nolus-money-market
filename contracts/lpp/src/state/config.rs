use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::Currency;
use finance::{percent::BoundToHundredPercent, price::Price};
use sdk::{
    cosmwasm_std::{Storage, Uint64},
    cw_storage_plus::Item,
};

use crate::{borrow::InterestRate, error::Result, msg::InstantiateMsg, nlpn::NLpn};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lpn_ticker: String,
    lease_code_id: Uint64,
    borrow_rate: InterestRate,
    min_utilization: BoundToHundredPercent,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    #[cfg(any(feature = "migration", test))]
    pub const fn new(
        lpn_ticker: String,
        lease_code_id: Uint64,
        borrow_rate: InterestRate,
        min_utilization: BoundToHundredPercent,
    ) -> Self {
        Self {
            lpn_ticker,
            lease_code_id,
            borrow_rate,
            min_utilization,
        }
    }

    pub fn lpn_ticker(&self) -> &str {
        &self.lpn_ticker
    }

    pub const fn lease_code_id(&self) -> Uint64 {
        self.lease_code_id
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
        Lpn: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
    }
}

macro_rules! update_param_impl {
    ($($method:ident($field:ident : $type:ty)),+ $(,)?) => {
        impl Config {
            $(
                pub fn $method(
                    storage: &mut dyn Storage,
                    $field: $type,
                ) -> Result<()> {
                    Self::STORAGE
                        .update(storage, |config: Self| {
                            Ok(Self {
                                $field,
                                ..config
                            })
                        })
                        .map(|_| ())
                }
            )+
        }
    };
}

update_param_impl!(
    update_lease_code(lease_code_id: Uint64),
    update_borrow_rate(borrow_rate: InterestRate),
    update_min_utilization(min_utilization: BoundToHundredPercent),
);

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        // 0 is a non-existing code id
        Self {
            lpn_ticker: msg.lpn_ticker,
            lease_code_id: Uint64::zero(),
            borrow_rate: msg.borrow_rate,
            min_utilization: msg.min_utilization,
        }
    }
}
