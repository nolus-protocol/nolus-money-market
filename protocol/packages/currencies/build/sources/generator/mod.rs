use std::iter;

use anyhow::Result;

use topology::HostCurrency;

use crate::{currencies_tree, protocol::Protocol};

use super::{
    DexCurrencies,
    resolved_currency::{CurrentModule, ResolvedCurrency},
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
    host_currency: &'host_currency HostCurrency,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
}

impl<'protocol, 'host_currency, 'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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
        host_currency: &'host_currency HostCurrency,
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
>(
    &'static_context StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
);

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
        Self(static_context)
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
    > {
        self.build(CurrentModule::Lease)
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
    > {
        self.build(CurrentModule::Lpn)
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
        true,
    > {
        self.build(CurrentModule::Native)
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
    > {
        self.build(CurrentModule::PaymentOnly)
    }

    #[inline]
    const fn build<const PAIRS_GROUP: bool>(
        &self,
        current_module: CurrentModule,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        PAIRS_GROUP,
    > {
        let Self(static_context) = *self;

        Generator {
            static_context,
            current_module,
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

pub(super) trait GroupMembers<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn group_members<'name>(&self, name: &'name str) -> Result<impl Iterator<Item = &'name str>>;
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, const PAIRS_GROUP: bool>
    GroupMembers<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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
    fn group_members<'name>(&self, name: &'name str) -> Result<impl Iterator<Item = &'name str>> {
        Ok([
            "
        Self::",
            name,
            " => ",
        ]
        .into_iter())
    }
}

pub(super) trait PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: &'parents currencies_tree::Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents currencies_tree::Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r;
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        '_,
        '_,
        '_,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
    >
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        _: &'name str,
        _: &'parents currencies_tree::Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents currencies_tree::Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r,
    {
        const { Ok(iter::empty()) }
    }
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<'_, '_, '_, 'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, true>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: &'parents currencies_tree::Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents currencies_tree::Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r,
    {
        pairs_group::pairs_group(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            name,
            parents.iter().copied(),
        )
    }
}

pub(super) trait InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn in_pool_with<'r, 'name, 'children, 'child>(
        &self,
        name: &'name str,
        children: &'children currencies_tree::Children<'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'children currencies_tree::Children<'child>>,
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
        children: &'children currencies_tree::Children<'child>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'children currencies_tree::Children<'child>>,
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
