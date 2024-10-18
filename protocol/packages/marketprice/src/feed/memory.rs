use std::slice::Iter;

use serde::{Deserialize, Serialize};

use currency::Currency;
use sdk::cosmwasm_std::Timestamp;

use super::{
    observation::{self, Observation},
    Observations,
};

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub(crate) struct InMemoryObservations<C, QuoteC>(Vec<Observation<C, QuoteC>>)
where
    C: 'static,
    QuoteC: 'static;

impl<'item, C, QuoteC> Observations<'item, C, QuoteC> for InMemoryObservations<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    type AsIter = Iter<'item, Observation<C, QuoteC>>;

    fn retain(&'item mut self, valid_since: Timestamp) {
        self.0.retain(observation::valid_since(valid_since))
    }

    fn register(&'item mut self, observation: Observation<C, QuoteC>) {
        self.0.push(observation);
    }

    fn as_iter(&'item self) -> Self::AsIter {
        self.0.iter()
    }
}

impl<C, QuoteC> Default for InMemoryObservations<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    fn default() -> Self {
        Self(Default::default())
    }
}
