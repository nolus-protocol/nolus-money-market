use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Timestamp;

use crate::error::PriceFeedsError;

use super::Observation;

pub trait ObservationsRead<C, QuoteC>
where
    C: 'static,
    QuoteC: 'static,
{
    fn len(&self) -> usize;

    fn as_iter(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<Observation<C, QuoteC>, PriceFeedsError>>,
        PriceFeedsError,
    >;
}

pub trait Observations<C, QuoteC>
where
    Self: ObservationsRead<C, QuoteC>,
    C: 'static,
    QuoteC: 'static,
{
    fn retain(&mut self, valid_since: Timestamp) -> Result<(), PriceFeedsError>;

    /// Register a newer observation
    ///
    /// The observation time must always flow monotonically forward!
    fn register(&mut self, observation: Observation<C, QuoteC>) -> Result<(), PriceFeedsError>;
}

pub trait ObservationsReadRepo {
    fn observations_read<C, QuoteC, G>(
        &self,
        c: CurrencyDTO<G>,
        quote_c: CurrencyDTO<G>,
    ) -> impl ObservationsRead<C, QuoteC>
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
        c: CurrencyDTO<G>,
        quote_c: CurrencyDTO<G>,
    ) -> impl Observations<C, QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group;
}
