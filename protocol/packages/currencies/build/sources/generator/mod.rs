use std::iter;

use anyhow::Result;
use topology::CurrencyDefinition;

use crate::{currencies_tree, protocol::Protocol};

use super::{
    resolved_currency::{CurrentModule, ResolvedCurrency},
    DexCurrencies,
};

mod pairs_group;

// TODO [precise capturing in trait definition]
//  Replace with precise capturing when it becomes available in trait
//  definitions.
pub(super) trait Captures<T>
where
    T: ?Sized,
{
}

impl<T, U> Captures<U> for T
where
    T: ?Sized,
    U: ?Sized,
{
}

pub(super) struct StaticContext<
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> {
    protocol: &'protocol Protocol,
    host_currency: &'host_currency CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
}

impl<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
    StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
{
    #[inline]
    pub const fn new(
        protocol: &'protocol Protocol,
        host_currency: &'host_currency CurrencyDefinition,
        dex_currencies: &'dex_currencies DexCurrencies<
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
    ) -> Self {
        Self {
            protocol,
            host_currency,
            dex_currencies,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Builder<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> {
    static_context: &'static_context StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
    Builder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
{
    #[inline]
    pub const fn new(
        static_context: &'static_context StaticContext<
            'protocol,
            'host_currency,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
    ) -> Self {
        Self { static_context }
    }

    #[inline]
    pub const fn lease(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
        true,
    > {
        Generator {
            static_context: self.static_context,
            current_module: CurrentModule::Lease,
        }
    }

    #[inline]
    pub const fn lpn(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
        false,
    > {
        Generator {
            static_context: self.static_context,
            current_module: CurrentModule::Lpn,
        }
    }

    #[inline]
    pub const fn native(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
        true,
    > {
        Generator {
            static_context: self.static_context,
            current_module: CurrentModule::Native,
        }
    }

    #[inline]
    pub const fn payment_only(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
        true,
    > {
        Generator {
            static_context: self.static_context,
            current_module: CurrentModule::PaymentOnly,
        }
    }
}

pub(super) struct Generator<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    const MAYBE_VISIT: bool,
    const PAIRS_GROUP: bool,
> {
    static_context: &'static_context StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
    current_module: CurrentModule,
}

pub(super) trait Resolver<'name, 'definition> {
    fn resolve(&self, ticker: &str) -> Result<ResolvedCurrency<'name, 'definition>>;
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'definition,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > Resolver<'dex_currencies, 'definition>
    for Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        PAIRS_GROUP,
    >
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

pub(super) trait MaybeVisit {
    const GENERATE: bool;
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > MaybeVisit
    for Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        PAIRS_GROUP,
    >
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    const GENERATE: bool = MAYBE_VISIT;
}

pub(super) trait PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn pairs_group<'r, 'name, 'children, 'child>(
        &self,
        name: &'name str,
        children: currencies_tree::Children<'children, 'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
            + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
            + Captures<&'name str>
            + Captures<currencies_tree::Children<'children, 'child>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r;
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
    > PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        false,
    >
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'children, 'child>(
        &self,
        _: &'name str,
        _: currencies_tree::Children<'children, 'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
            + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
            + Captures<&'name str>
            + Captures<currencies_tree::Children<'children, 'child>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
    {
        const { Ok(iter::empty()) }
    }
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
    > PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        true,
    >
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'children, 'child>(
        &self,
        name: &'name str,
        children: currencies_tree::Children<'children, 'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
            + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
            + Captures<&'name str>
            + Captures<currencies_tree::Children<'children, 'child>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
    {
        pairs_group::pairs_group(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            name,
            children.iter().copied(),
        )
    }
}

pub(super) trait InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn in_pool_with<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: currencies_tree::Parents<'parents, 'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
            + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
            + Captures<&'name str>
            + Captures<currencies_tree::Parents<'parents, 'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r;
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        PAIRS_GROUP,
    >
{
    #[inline]
    fn in_pool_with<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: currencies_tree::Parents<'parents, 'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
            + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
            + Captures<&'name str>
            + Captures<currencies_tree::Parents<'parents, 'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
    {
        let current_module = self.current_module;

        let protocol = self.static_context.protocol;

        let host_currency = self.static_context.host_currency;

        let dex_currencies = self.static_context.dex_currencies;

        let parents = parents.iter().copied();

        parents
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
                        " {}
",
                    ]
                })
            })
            .collect::<Result<_, _>>()
            .map(Vec::into_iter)
            .map(Iterator::flatten)
    }
}
