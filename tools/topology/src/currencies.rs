use std::collections::BTreeMap;

use serde::Deserialize;

use crate::currency::{self, Currency};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub(crate) struct Currencies(BTreeMap<currency::Id, Currency>);

impl Currencies {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&currency::Id, &Currency)> + '_ + use<'_> {
        self.0.iter()
    }

    #[inline]
    pub fn get<'self_>(&'self_ self, currency: &currency::Id) -> Option<&'self_ Currency> {
        self.0.get(currency)
    }
}
