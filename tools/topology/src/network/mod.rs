use serde::Deserialize;

use crate::{currencies::Currencies, newtype};

pub(crate) use self::host::Host as HostNetwork;

mod host;

newtype::define!(
    #[derive(Debug, Clone, Deserialize)]
    #[serde(transparent)]
    pub(crate) Id(String)
    as [String, str]
);

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
