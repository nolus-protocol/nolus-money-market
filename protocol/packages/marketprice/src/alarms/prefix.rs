use currency::{CurrencyDTO, Group, SymbolStatic, Tickers};

use super::NormalizedPrice;

pub trait Prefix {
    fn first_key(&self) -> SymbolStatic;
}

impl<G> Prefix for CurrencyDTO<G>
where
    G: Group,
{
    fn first_key(&self) -> SymbolStatic {
        self.into_symbol::<Tickers<G>>()
    }
}

impl<G> Prefix for NormalizedPrice<G>
where
    G: Group,
{
    fn first_key(&self) -> SymbolStatic {
        self.0.currency().first_key()
    }
}
