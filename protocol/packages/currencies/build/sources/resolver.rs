use anyhow::Result;

use super::resolved_currency::ResolvedCurrency;

pub(super) trait Resolver<'name, 'definition> {
    fn resolve(&self, ticker: &str) -> Result<ResolvedCurrency<'name, 'definition>>;
}
