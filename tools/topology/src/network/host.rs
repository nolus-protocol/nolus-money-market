use serde::Deserialize;

use crate::currency;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) struct Host {
    name: super::Id,
    currency: Currency,
}

impl Host {
    #[inline]
    pub const fn name(&self) -> &super::Id {
        &self.name
    }

    #[inline]
    pub const fn currency(&self) -> &Currency {
        &self.currency
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Currency {
    id: currency::Id,
    native: currency::Native,
}

impl Currency {
    #[inline]
    pub const fn id(&self) -> &currency::Id {
        &self.id
    }

    #[inline]
    pub const fn native(&self) -> &currency::Native {
        &self.native
    }
}
