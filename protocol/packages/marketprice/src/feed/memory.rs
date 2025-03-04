use sdk::cosmwasm_std::Timestamp;

use crate::error::PriceFeedsError;

use super::{Observations, ObservationsRead, observation::Observation};

pub(crate) struct InMemoryObservations<C, QuoteC>(Vec<Observation<C, QuoteC>>)
where
    C: 'static,
    QuoteC: 'static;

impl<C, QuoteC> InMemoryObservations<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }
}

impl<C, QuoteC> ObservationsRead for InMemoryObservations<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    type C = C;

    type QuoteC = QuoteC;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn as_iter(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<Observation<C, QuoteC>, PriceFeedsError>>,
        PriceFeedsError,
    > {
        Ok(self.0.clone().into_iter().map(Result::Ok))
    }
}

impl<C, QuoteC> Observations for InMemoryObservations<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn retain(&mut self, valid_since: &Timestamp) -> Result<(), PriceFeedsError> {
        self.0.retain(|o| o.valid_since(valid_since));
        Ok(())
    }

    fn register(&mut self, observation: Observation<C, QuoteC>) -> Result<(), PriceFeedsError> {
        self.0.push(observation);
        Ok(())
    }
}
