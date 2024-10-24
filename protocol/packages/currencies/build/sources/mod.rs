use std::{collections::BTreeMap, io::Write, iter, path::Path};

use anyhow::Result;

use topology::CurrencyDefinition;

use crate::{
    currencies_tree::{self, CurrenciesTree},
    protocol::Protocol,
};

use self::resolved_currency::{CurrentModule, ResolvedCurrency};

mod in_pool_with;
mod multiple_currency;
mod pairs_group;
mod resolved_currency;
mod stable;

pub(super) fn write<BuildReport>(
    mut build_report: BuildReport,
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &DexCurrencies<'_, '_>,
    currencies_tree: &CurrenciesTree<'_, '_, '_, '_>,
) -> Result<()>
where
    BuildReport: Write,
{
    let multiple_currency_source_generator =
        multiple_currency::SourcesGenerator::new(currencies_tree);

    let static_context = &GeneratorStaticContext {
        protocol,
        host_currency,
        dex_currencies,
    };

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("lease.rs"),
        &GeneratorImpl::with_pairs_group(static_context, CurrentModule::Lease),
        dex_currencies
            .keys()
            .copied()
            .filter(|&key| protocol.lease_currencies_tickers.contains(key)),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("lpn.rs"),
        &GeneratorImpl::without_pairs_group(static_context, CurrentModule::Lpn),
        iter::once(&*protocol.lpn_ticker),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("native.rs"),
        &GeneratorImpl::with_pairs_group(static_context, CurrentModule::Native),
        iter::once(host_currency.ticker()),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("payment_only.rs"),
        &GeneratorImpl::with_pairs_group(static_context, CurrentModule::PaymentOnly),
        dex_currencies.keys().copied().filter(|&key| {
            !(key == protocol.lpn_ticker || protocol.lease_currencies_tickers.contains(key))
        }),
    )?;

    stable::write(build_report, output_directory, protocol, dex_currencies)
}

type DexCurrencies<'ticker, 'currency_definition> =
    BTreeMap<&'ticker str, (String, &'currency_definition CurrencyDefinition)>;

trait Captures<T>
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

trait Resolver<'name, 'definition> {
    fn resolve(&self, ticker: &str) -> Result<ResolvedCurrency<'name, 'definition>>;
}

trait Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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

struct GeneratorStaticContext<
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

struct GeneratorImpl<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    const PAIRS_GROUP: bool,
> {
    static_context: &'static_context GeneratorStaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
    current_module: CurrentModule,
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
    GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
    >
{
    const fn without_pairs_group(
        static_context: &'static_context GeneratorStaticContext<
            'protocol,
            'host_currency,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
        current_module: CurrentModule,
    ) -> Self {
        Self {
            static_context,
            current_module,
        }
    }
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
    GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
    >
{
    const fn with_pairs_group(
        static_context: &'static_context GeneratorStaticContext<
            'protocol,
            'host_currency,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
        current_module: CurrentModule,
    ) -> Self {
        Self {
            static_context,
            current_module,
        }
    }
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'definition,
        'dex_currency_ticker,
        'dex_currency_definition,
        const PAIRS_GROUP: bool,
    > Resolver<'dex_currencies, 'definition>
    for GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        PAIRS_GROUP,
    >
where
    'host_currency: 'definition,
    'dex_currencies: 'definition,
{
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

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    > Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
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
        in_pool_with::in_pool_with(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            name,
            parents.iter().copied(),
        )
    }
}

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    > Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
    >
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
        in_pool_with::in_pool_with(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            name,
            parents.iter().copied(),
        )
    }
}
