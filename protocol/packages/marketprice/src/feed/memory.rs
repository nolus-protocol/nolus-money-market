use std::marker::PhantomData;

use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Timestamp;

use crate::error::PriceFeedsError;

use super::{
    observation::{self, Observation},
    observations::{ObservationsReadRepo, ObservationsRepo},
    Observations, ObservationsRead,
};

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
    fn retain(&mut self, valid_since: Timestamp) -> Result<(), PriceFeedsError> {
        self.0.retain(observation::valid_since(valid_since));
        Ok(())
    }

    fn register(&mut self, observation: Observation<C, QuoteC>) -> Result<(), PriceFeedsError> {
        self.0.push(observation);
        Ok(())
    }
}

// pub struct InMemoryRepo(HashMap<(CurrencyDTO<G>, CurrencyDTO<G>), InMemoryObservations<>>);
pub struct InMemoryRepo<G>(PhantomData<G>);

impl<G> InMemoryRepo<G> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G> ObservationsReadRepo for InMemoryRepo<G>
where
    G: Group,
{
    type Group = G;

    fn observations_read<C, QuoteC>(
        &self,
        _c: &CurrencyDTO<Self::Group>,
        _quote_c: &CurrencyDTO<Self::Group>,
    ) -> impl ObservationsRead<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group,
    {
        InMemoryObservations::new()
    }
}

impl<G> ObservationsRepo for InMemoryRepo<G>
where
    G: Group,
{
    fn observations<C, QuoteC>(
        &mut self,
        _c: &CurrencyDTO<Self::Group>,
        _quote_c: &CurrencyDTO<Self::Group>,
    ) -> impl Observations<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group,
    {
        InMemoryObservations::new()
    }
}

impl<G> Default for InMemoryRepo<G> {
    fn default() -> Self {
        Self::new()
    }
}
