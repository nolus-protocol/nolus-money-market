use std::{borrow::Cow, iter};

use anyhow::{Context as _, Result};
use either::Either;

use crate::subtype_lifetime::SubtypeLifetime;

use super::{super::generator, FinalizedSources, Writer};

use self::currency_definition::CurrencyDefinition;

mod currency_definition;

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, 'currencies_tree, 'parent>
    Writer<'currencies_tree, '_, 'parent, '_, '_>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    pub(super) fn generate_sources<'r, 'generator, 'ticker, Generator, Tickers>(
        &self,
        generator: &'generator Generator,
        mut tickers: Tickers,
    ) -> Result<
        FinalizedSources<
            impl Iterator<Item = Cow<'r, str>>
            + use<
                'r,
                'dex_currencies,
                'dex_currency_ticker,
                'dex_currency_definition,
                'currencies_tree,
                'parent,
                'generator,
                Generator,
                Tickers,
            >,
        >,
    >
    where
        'dex_currencies: 'r,
        'parent: 'r,
        'ticker: 'r,
        Generator: generator::Resolver<'dex_currencies, 'dex_currencies>
            + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
            + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
        Tickers: Iterator<Item = &'ticker str>,
    {
        if let Some(head_ticker) = tickers.next() {
            with_currencies_sources(
                CurrencyDefinition::new(self.currencies_tree, generator),
                head_ticker,
                tickers,
            )
            .map(|non_finalized_sources| {
                non_finalized_sources.map_currency_definitions(Either::Left)
            })
        } else {
            Ok(NonFinalizedSources::new(0, const { iter::empty() })
                .map_currency_definitions(Either::Right))
        }
        .map(NonFinalizedSources::finalize)
    }
}

fn with_currencies_sources<
    'r,
    'host_currency,
    'dex_currencies,
    'definition,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'parent,
    'generator,
    'ticker,
    Generator,
    Tickers,
>(
    definition: CurrencyDefinition<'currencies_tree, '_, 'parent, '_, '_, 'generator, Generator>,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    NonFinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
        + use<
            'r,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
            'currencies_tree,
            'parent,
            'generator,
            Generator,
            Tickers,
        >,
    >,
>
where
    'host_currency: 'definition,
    'dex_currencies: 'definition,
    'definition: 'r,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    'parent: 'r,
    'ticker: 'r,
    Generator: generator::Resolver<'dex_currencies, 'definition>
        + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
    Tickers: Iterator<Item = &'ticker str>,
{
    iter::once(head_ticker)
        .chain(tail_tickers)
        .map(|ticker| definition.generate_entry(ticker))
        .try_fold(
            NonFinalizedSources::new(0, vec![]),
            NonFinalizedSources::try_fold,
        )
        .map(|non_finalized_sources| {
            non_finalized_sources.map_currency_definitions(|currency_definitions| {
                currency_definitions.into_iter().flatten()
            })
        })
}

struct NonFinalizedSources<CurrencyDefinitions> {
    currencies_count: usize,
    currency_definitions: CurrencyDefinitions,
}

impl<CurrencyDefinitions> NonFinalizedSources<CurrencyDefinitions> {
    #[inline]
    const fn new(currencies_count: usize, currency_definitions: CurrencyDefinitions) -> Self {
        Self {
            currencies_count,
            currency_definitions,
        }
    }

    #[inline]
    fn map_currency_definitions<F, R>(self, f: F) -> NonFinalizedSources<R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<'maybe_visit, CurrencyDefinition> NonFinalizedSources<Vec<CurrencyDefinition>> {
    fn try_fold(
        Self {
            currencies_count,
            mut currency_definitions,
        }: Self,
        element: Result<CurrencyDefinition>,
    ) -> Result<Self> {
        currencies_count
            .checked_add(1)
            .context("Unable to increment generated currencies count! Overflowing beyond maximum capacity!")
            .and_then(|currencies_count| {
                element.map(|currency_definition| {
                    currency_definitions.push(currency_definition);

                    Self {
                        currencies_count,
                        currency_definitions,
                    }
                })
            })
    }
}

impl<'r, 'maybe_visit, 'currency_definition, CurrencyDefinitions>
    NonFinalizedSources<CurrencyDefinitions>
where
    'maybe_visit: 'r,
    'currency_definition: 'r,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
{
    #[inline]
    fn finalize(
        self,
    ) -> FinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
        + use<'r, 'maybe_visit, 'currency_definition, CurrencyDefinitions>,
    > {
        let Self {
            currencies_count,
            currency_definitions,
        } = self;

        FinalizedSources {
            currencies_count,
            sources: iter::once(Cow::Borrowed("// @generated\n"))
                .chain(currency_definitions.map(SubtypeLifetime::subtype)),
        }
    }
}
