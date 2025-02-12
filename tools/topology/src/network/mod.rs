use std::borrow::Borrow;

use serde::Deserialize;

use crate::{currencies::Currencies, dex::Dexes};

pub(crate) use self::host::Host;

mod host;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub(crate) struct Id(String);

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
#[serde(from = "Raw")]
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
    fn from(Raw { currencies, .. }: Raw) -> Self {
        Self { currencies }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct Raw {
    currencies: Currencies,
    #[serde(default, rename = "dexes")]
    _dexes: Dexes,
}
