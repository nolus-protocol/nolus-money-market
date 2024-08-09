use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

use serde::Deserialize;

use crate::currencies::Currencies;

pub(crate) use self::host::Host as HostNetwork;

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
pub(crate) struct Network {
    currencies: Currencies,
}

impl Network {
    #[inline]
    pub const fn currencies(&self) -> &Currencies {
        &self.currencies
    }
}
