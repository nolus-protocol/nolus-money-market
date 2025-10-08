use std::{borrow::Cow, iter};

use anyhow::{Context as _, Result};
use either::Either;

use crate::subtype_lifetime::SubtypeLifetime;

use super::{super::generator, FinalizedSources, Writer};

use self::currency_definition::{CurrencyDefinition, GeneratedEntry, GeneratedEntryResult};

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
            + generator::GroupMembers<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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
                non_finalized_sources
                    .map_currencies(Some)
                    .map_currency_definitions(Either::Left)
            })
        } else {
            const {
                Ok(NonFinalizedSources {
                    currencies_count: 0,
                    currencies: None,
                    currency_definitions: Either::Right(iter::empty()),
                })
            }
        }
        .map(NonFinalizedSources::finalize)
    }
}

type WithCurrenciesSourcesResult<Currencies, CurrencyDefinitions> =
    Result<NonFinalizedSources<Currencies, CurrencyDefinitions>>;

trait NestedIterator: Iterator<Item: Iterator<Item = Self::NestedItem>> {
    type NestedItem;
}

impl<T> NestedIterator for T
where
    T: Iterator + ?Sized,
    T::Item: Iterator,
{
    type NestedItem = <T::Item as Iterator>::Item;
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
) -> WithCurrenciesSourcesResult<
    impl NestedIterator<NestedItem = &'r str> + use<'r, Generator, Tickers>,
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
        + generator::GroupMembers<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
    Tickers: Iterator<Item = &'ticker str>,
{
    #[inline]
    fn flatten_iter_vec<I>(v: Vec<I>) -> impl Iterator<Item = I::Item>
    where
        I: IntoIterator,
    {
        v.into_iter().flatten()
    }

    iter::once(head_ticker)
        .chain(tail_tickers)
        .map(|ticker| definition.generate_entry(ticker))
        .try_fold(NonFinalizedSources::empty(), NonFinalizedSources::try_fold)
        .map(|non_finalized_sources| {
            non_finalized_sources
                .map_currencies(IntoIterator::into_iter)
                .map_currency_definitions(flatten_iter_vec)
        })
}

struct NonFinalizedSources<Currencies, CurrencyDefinitions> {
    currencies_count: usize,
    currencies: Currencies,
    currency_definitions: CurrencyDefinitions,
}

impl<Currency, CurrencyDefinition> NonFinalizedSources<Vec<Currency>, Vec<CurrencyDefinition>> {
    #[inline]
    const fn empty() -> Self {
        const {
            Self {
                currencies_count: 0,
                currencies: vec![],
                currency_definitions: vec![],
            }
        }
    }
}

impl<Currencies, CurrencyDefinitions> NonFinalizedSources<Currencies, CurrencyDefinitions> {
    #[inline]
    fn map_currencies<F, R>(self, f: F) -> NonFinalizedSources<R, CurrencyDefinitions>
    where
        F: FnOnce(Currencies) -> R,
    {
        let Self {
            currencies_count,
            currencies,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currencies: f(currencies),
            currency_definitions,
        }
    }

    #[inline]
    fn map_currency_definitions<F, R>(self, f: F) -> NonFinalizedSources<Currencies, R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            currencies,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currencies,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<Currency, CurrencyDefinition> NonFinalizedSources<Vec<Currency>, Vec<CurrencyDefinition>> {
    fn try_fold(self, element: GeneratedEntryResult<Currency, CurrencyDefinition>) -> Result<Self> {
        let Self {
            currencies_count,
            mut currencies,
            mut currency_definitions,
        } = self;

        currencies_count
            .checked_add(1)
            .context("Unable to increment generated currencies count! Overflowing beyond maximum capacity!")
            .and_then(|currencies_count| {
                element.map(|GeneratedEntry { currency, currency_definition }| {
                    currencies.push(currency);

                    currency_definitions.push(currency_definition);

                    Self {
                        currencies_count,
                        currencies,
                        currency_definitions,
                    }
                })
            })
    }
}

impl<'r, 'currency, 'currency_definition, Currencies, CurrencyDefinitions>
    NonFinalizedSources<Option<Currencies>, CurrencyDefinitions>
where
    'currency: 'r,
    'currency_definition: 'r,
    Currencies: NestedIterator<NestedItem = &'currency str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
{
    #[inline]
    fn finalize(
        self,
    ) -> FinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
        + use<'r, 'currency, 'currency_definition, Currencies, CurrencyDefinitions>,
    > {
        let Self {
            currencies_count,
            currencies,
            currency_definitions,
        } = self;

        FinalizedSources {
            currencies_count,
            sources: iter::once(
                "// @generated

pub(super) type Members =",
            )
            .chain(currencies.map_or_else(
                || Either::Right(iter::once(" ()")),
                |currencies| {
                    Either::Left(currencies.flat_map(|currency| {
                        iter::once(" (")
                            .chain(currency.map(SubtypeLifetime::subtype))
                            .chain(iter::once(","))
                    }))
                },
            ))
            .chain(iter::repeat_n(")", currencies_count))
            .chain(iter::once(
                ";

pub(super) mod definitions {",
            ))
            .map(Cow::Borrowed)
            .chain(
                currency_definitions
                    .into_iter()
                    .map(SubtypeLifetime::subtype),
            )
            .chain(iter::once(Cow::Borrowed("}\n"))),
        }
    }
}
