use crate::{
    CurrencyDef, Group, SymbolRef, Tickers,
    error::{Error, Result},
};

pub fn expect_received<C, G>(expected: SymbolRef<'_>, received: SymbolRef<'_>) -> Result<()>
where
    C: CurrencyDef,
    G: Group,
{
    if expected == received {
        Ok(())
    } else {
        Err(Error::unexpected_symbol::<_, Tickers<G>>(
            received.to_owned(),
            C::dto().definition(),
        ))
    }
}
