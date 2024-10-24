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
    let static_context = &GeneratorStaticContext {
        protocol,
        host_currency,
        dex_currencies,
    };

    let builder = GeneratorBuilder::new(static_context);

    let multiple_currency_source_generator =
        multiple_currency::SourcesGenerator::new(currencies_tree);

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("lease.rs"),
        &builder
            .with_current_module(CurrentModule::Lease)
            .with_maybe_visit::<true>()
            .with_pairs_group::<true>()
            .build(),
        dex_currencies
            .keys()
            .copied()
            .filter(|&key| protocol.lease_currencies_tickers.contains(key)),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("lpn.rs"),
        &builder
            .with_current_module(CurrentModule::Lpn)
            .with_maybe_visit::<false>()
            .with_pairs_group::<false>()
            .build(),
        iter::once(&*protocol.lpn_ticker),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("native.rs"),
        &builder
            .with_current_module(CurrentModule::Native)
            .with_maybe_visit::<false>()
            .with_pairs_group::<true>()
            .build(),
        iter::once(host_currency.ticker()),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("payment_only.rs"),
        &builder
            .with_current_module(CurrentModule::PaymentOnly)
            .with_maybe_visit::<true>()
            .with_pairs_group::<true>()
            .build(),
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

trait MaybeVisitGenerator {
    const GENERATE: bool;
}

trait PairsGroupGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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

trait InPoolWithGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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

#[derive(Clone, Copy)]
struct GeneratorBuilder<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    CurrentModule,
    const MAYBE_VISIT_SET: bool,
    const MAYBE_VISIT_VALUE: bool,
    const PAIRS_GROUP_SET: bool,
    const PAIRS_GROUP_VALUE: bool,
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
    GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        (),
        false,
        false,
        false,
        false,
    >
{
    const fn new(
        static_context: &'static_context GeneratorStaticContext<
            'protocol,
            'host_currency,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
    ) -> Self {
        Self {
            static_context,
            current_module: (),
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
        const PAIRS_GROUP_SET: bool,
        const PAIRS_GROUP_VALUE: bool,
        const IN_POOL_WITH_SET: bool,
        const IN_POOL_WITH_VALUE: bool,
    >
    GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        (),
        PAIRS_GROUP_SET,
        PAIRS_GROUP_VALUE,
        IN_POOL_WITH_SET,
        IN_POOL_WITH_VALUE,
    >
{
    fn with_current_module(
        self,
        current_module: CurrentModule,
    ) -> GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        PAIRS_GROUP_SET,
        PAIRS_GROUP_VALUE,
        IN_POOL_WITH_SET,
        IN_POOL_WITH_VALUE,
    > {
        let Self {
            static_context,
            current_module: (),
        } = self;

        GeneratorBuilder {
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
        CurrentModule,
        const PAIRS_GROUP_SET: bool,
        const PAIRS_GROUP_VALUE: bool,
    >
    GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        false,
        false,
        PAIRS_GROUP_SET,
        PAIRS_GROUP_VALUE,
    >
{
    fn with_maybe_visit<const MAYBE_VISIT_VALUE: bool>(
        self,
    ) -> GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        true,
        MAYBE_VISIT_VALUE,
        PAIRS_GROUP_SET,
        PAIRS_GROUP_VALUE,
    > {
        let Self {
            static_context,
            current_module,
        } = self;

        GeneratorBuilder {
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
        CurrentModule,
        const MAYBE_VISIT_SET: bool,
        const MAYBE_VISIT_VALUE: bool,
    >
    GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        MAYBE_VISIT_SET,
        MAYBE_VISIT_VALUE,
        false,
        false,
    >
{
    fn with_pairs_group<const PAIRS_GROUP_VALUE: bool>(
        self,
    ) -> GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        MAYBE_VISIT_SET,
        MAYBE_VISIT_VALUE,
        true,
        PAIRS_GROUP_VALUE,
    > {
        let Self {
            static_context,
            current_module,
        } = self;

        GeneratorBuilder {
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
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    >
    GeneratorBuilder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        CurrentModule,
        true,
        MAYBE_VISIT,
        true,
        PAIRS_GROUP,
    >
{
    fn build(
        self,
    ) -> GeneratorImpl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        MAYBE_VISIT,
        PAIRS_GROUP,
    > {
        let Self {
            static_context,
            current_module,
        } = self;

        GeneratorImpl {
            static_context,
            current_module,
        }
    }
}

struct GeneratorImpl<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    const MAYBE_VISIT: bool,
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
        'definition,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > Resolver<'dex_currencies, 'definition>
    for GeneratorImpl<
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
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > MaybeVisitGenerator
    for GeneratorImpl<
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

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
    > PairsGroupGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for GeneratorImpl<
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
    > PairsGroupGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for GeneratorImpl<
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

impl<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        const MAYBE_VISIT: bool,
        const PAIRS_GROUP: bool,
    > InPoolWithGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for GeneratorImpl<
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
