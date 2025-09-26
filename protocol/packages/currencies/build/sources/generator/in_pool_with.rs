use anyhow::Result;

use crate::currencies_tree::Children;

use super::{
    super::{DexCurrencies, ResolvedCurrency},
    Captures, Generator,
};

pub(in super::super) trait InPoolWith<
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn in_pool_with<'r, 'name, 'children, 'child>(
        &self,
        name: &'name str,
        children: &'children Children<'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'children Children<'child>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r;
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, const PAIRS_GROUP: bool>
    InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        '_,
        '_,
        '_,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        PAIRS_GROUP,
    >
{
    #[inline]
    fn in_pool_with<'r, 'name, 'children, 'child>(
        &self,
        name: &'name str,
        children: &'children Children<'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'children Children<'child>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
    {
        let current_module = self.current_module;

        let protocol = self.static_context.protocol;

        let host_currency = self.static_context.host_currency;

        let dex_currencies = self.static_context.dex_currencies;

        children
            .iter()
            .copied()
            .map(|ticker| {
                ResolvedCurrency::resolve(
                    current_module,
                    protocol,
                    host_currency,
                    dex_currencies,
                    ticker,
                )
                .map(|resolved| {
                    [
                        "
    impl currency::InPoolWith<",
                        resolved.module(),
                        "::",
                        resolved.name(),
                        "> for ",
                        name,
                        " {}\n",
                    ]
                })
            })
            .collect::<Result<_, _>>()
            .map(Vec::into_iter)
            .map(Iterator::flatten)
    }
}
