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
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Network {
    currencies: Currencies,
    #[serde(default)]
    dexes: Dexes,
}

impl Network {
    #[inline]
    pub const fn currencies(&self) -> &Currencies {
        &self.currencies
    }

    #[inline]
    pub const fn dexes(&self) -> &Dexes {
        &self.dexes
    }
}
