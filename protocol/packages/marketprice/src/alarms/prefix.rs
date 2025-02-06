use currency::{CurrencyDTO, Group};

use super::NormalizedPrice;

pub trait Prefix {
    type G: Group;

    fn first_key(&self) -> CurrencyDTO<Self::G>;
}

impl<G> Prefix for CurrencyDTO<G>
where
    G: Group,
{
    type G = G;

    fn first_key(&self) -> CurrencyDTO<Self::G> {
        *self
    }
}

impl<G> Prefix for NormalizedPrice<G>
where
    G: Group,
{
    type G = G;

    fn first_key(&self) -> CurrencyDTO<Self::G> {
        self.0.currency()
    }
}
