use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Timestamp;

use crate::error::PriceFeedsError;

use super::Observation;

pub trait ObservationsRead {
    type C: 'static;

    type QuoteC: 'static;

    fn len(&self) -> usize;

    fn as_iter(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<Observation<Self::C, Self::QuoteC>, PriceFeedsError>>,
        PriceFeedsError,
    >;
}

pub trait Observations
where
    Self: ObservationsRead,
    Self::C: 'static,
    Self::QuoteC: 'static,
{
    fn retain(&mut self, valid_since: Timestamp) -> Result<(), PriceFeedsError>;

    /// Register a newer observation
    ///
    /// The observation time must always flow monotonically forward!
    fn register(
        &mut self,
        observation: Observation<Self::C, Self::QuoteC>,
    ) -> Result<(), PriceFeedsError>;
}

pub trait ObservationsReadRepo {
    fn observations_read<C, QuoteC, G>(
        &self,
        c: &CurrencyDTO<G>,
        quote_c: &CurrencyDTO<G>,
    ) -> impl ObservationsRead<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group;
}

pub trait ObservationsRepo
where
    Self: ObservationsReadRepo,
{
    fn observations<C, QuoteC, G>(
        &mut self,
        c: &CurrencyDTO<G>,
        quote_c: &CurrencyDTO<G>,
    ) -> impl Observations<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group;
}
