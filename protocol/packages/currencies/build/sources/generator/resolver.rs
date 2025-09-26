use anyhow::Result;

use super::{super::ResolvedCurrency, Generator};

pub(in super::super) trait Resolver<'name, 'definition> {
    fn resolve(&self, ticker: &str) -> Result<ResolvedCurrency<'name, 'definition>>;
}

impl<'host_currency, 'dex_currencies, 'definition, const PAIRS_GROUP: bool>
    Resolver<'dex_currencies, 'definition>
    for Generator<'_, '_, 'host_currency, 'dex_currencies, '_, '_, PAIRS_GROUP>
where
    'host_currency: 'definition,
    'dex_currencies: 'definition,
{
    #[inline]
    fn resolve(&self, ticker: &str) -> Result<ResolvedCurrency<'dex_currencies, 'definition>> {
        ResolvedCurrency::resolve(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            ticker,
        )
    }
}
