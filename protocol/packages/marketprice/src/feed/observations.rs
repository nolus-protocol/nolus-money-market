use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Timestamp;

use crate::error::PriceFeedsError;

use super::Observation;

pub type Result<T> = std::result::Result<T, PriceFeedsError>;

pub trait ObservationsRead {
    type C: 'static;

    type QuoteC: 'static;

    fn len(&self) -> usize;

    fn as_iter(&self) -> Result<impl Iterator<Item = Result<Observation<Self::C, Self::QuoteC>>>>;
}

pub trait Observations
where
    Self: ObservationsRead,
    Self::C: 'static,
    Self::QuoteC: 'static,
{
    fn retain(&mut self, valid_since: Timestamp) -> Result<()>;

    /// Register a newer observation
    ///
    /// The observation time must always flow monotonically forward!
    fn register(&mut self, observation: Observation<Self::C, Self::QuoteC>) -> Result<()>;
}

pub trait ObservationsReadRepo {
    type Group: Group;

    fn observations_read<C, QuoteC>(
        &self,
        c: &CurrencyDTO<Self::Group>,
        quote_c: &CurrencyDTO<Self::Group>,
    ) -> impl ObservationsRead<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static;
}

pub trait ObservationsRepo
where
    Self: ObservationsReadRepo,
{
    fn observations<C, QuoteC>(
        &mut self,
        c: &CurrencyDTO<Self::Group>,
        quote_c: &CurrencyDTO<Self::Group>,
    ) -> impl Observations<C = C, QuoteC = QuoteC>
    where
        C: 'static,
        QuoteC: 'static;
}
