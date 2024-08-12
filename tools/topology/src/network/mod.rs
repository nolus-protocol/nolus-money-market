use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

use serde::Deserialize;

use crate::currencies::Currencies;

pub(crate) use self::host::Host as HostNetwork;

use self::amm_pool::AmmPool;

mod amm_pool;
mod host;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub(crate) struct Id(String);

impl Hash for Id {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Id {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(from = "self::Raw")]
pub(crate) struct Network {
    currencies: Currencies,
}

impl Network {
    #[inline]
    pub const fn currencies(&self) -> &Currencies {
        &self.currencies
    }
}

impl From<Raw> for Network {
    #[inline]
    fn from(Raw { currencies, .. }: Raw) -> Self {
        Self { currencies }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Raw {
    currencies: Currencies,
    #[serde(default, rename = "amm_pools")]
    _amm_pools: Vec<AmmPool>,
}
