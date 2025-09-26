use std::{borrow::Cow, iter};

use anyhow::{Context as _, Result};
use either::Either;

use crate::subtype_lifetime::SubtypeLifetime;

use super::{
    super::generator::{self, GroupMemberEntry},
    FinalizedSources, Writer,
};

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
            + generator::GroupMember<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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
                    .map_variants(|variants| Either::Left(variants.into_iter()))
                    .map_first(|first| first.map(IntoIterator::into_iter))
                    .map_next(|next| {
                        next.map(|(head, middle, tail)| {
                            head.into_iter()
                                .chain(middle.into_iter().flatten())
                                .chain(tail)
                        })
                    })
                    .map_filter_map(|filter_map| Some(filter_map.into_iter()))
                    .map_find_map(|find_map| Some(find_map.into_iter()))
                    .map_currency_definitions(|currency_definitions| {
                        Either::Left(currency_definitions.into_iter())
                    })
            })
        } else {
            const {
                Ok(NonFinalizedSources {
                    currencies_count: 0,
                    variants: Either::Right(iter::empty()),
                    first: None,
                    next: None,
                    filter_map: None,
                    find_map: None,
                    currency_definitions: Either::Right(iter::empty()),
                })
            }
        }
        .map(NonFinalizedSources::finalize)
    }
}

type WithCurrenciesSourcesResult<Variants, First, Next, FilterMap, FindMap, CurrencyDefinitions> =
    Result<
        NonFinalizedSources<
            Variants,
            Option<First>,
            Option<Next>,
            FilterMap,
            FindMap,
            CurrencyDefinitions,
        >,
    >;

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
    impl IntoIterator<Item = &'dex_currencies str>
    + use<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'generator,
        Generator,
        Tickers,
    >,
    impl IntoIterator<Item = &'dex_currencies str>
    + use<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'generator,
        Generator,
        Tickers,
    >,
    (
        impl IntoIterator<Item = &'dex_currencies str>
        + use<
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
            'generator,
            Generator,
            Tickers,
        >,
        Vec<
            impl IntoIterator<Item = &'dex_currencies str>
            + use<
                'dex_currencies,
                'dex_currency_ticker,
                'dex_currency_definition,
                'generator,
                Generator,
                Tickers,
            >,
        >,
        impl IntoIterator<Item = &'dex_currencies str>
        + use<
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
            'generator,
            Generator,
            Tickers,
        >,
    ),
    impl IntoIterator<Item = &'dex_currencies str>
    + use<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'generator,
        Generator,
        Tickers,
    >,
    impl IntoIterator<Item = &'dex_currencies str>
    + use<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'generator,
        Generator,
        Tickers,
    >,
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
        + generator::GroupMember<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
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

    fn chain_whitespace<'r, I>(iter: I) -> impl Iterator<Item = &'r str>
    where
        I: IntoIterator<Item = &'r str>,
    {
        iter.into_iter().chain(iter::once(
            "
        ",
        ))
    }

    iter::once(head_ticker)
        .chain(tail_tickers)
        .map(|ticker| definition.generate_entry(ticker))
        .try_fold(NonFinalizedSources::empty(), NonFinalizedSources::try_fold)
        .map(|non_finalized_sources| {
            non_finalized_sources
                .map_variants(flatten_iter_vec)
                .map_filter_map(flatten_iter_vec)
                .map_filter_map(chain_whitespace)
                .map_find_map(flatten_iter_vec)
                .map_find_map(chain_whitespace)
                .map_currency_definitions(flatten_iter_vec)
        })
}

struct NonFinalizedSources<Variants, First, Next, FilterMap, FindMap, CurrencyDefinitions> {
    currencies_count: usize,
    variants: Variants,
    first: First,
    next: Next,
    filter_map: FilterMap,
    find_map: FindMap,
    currency_definitions: CurrencyDefinitions,
}

impl<Variants, First, Next, FilterMap, FindMap, CurrencyDefinitions>
    NonFinalizedSources<
        Vec<Variants>,
        Option<First>,
        Option<Next>,
        Vec<FilterMap>,
        Vec<FindMap>,
        Vec<CurrencyDefinitions>,
    >
{
    #[inline]
    const fn empty() -> Self {
        const {
            Self {
                currencies_count: 0,
                variants: vec![],
                first: None,
                next: None,
                filter_map: vec![],
                find_map: vec![],
                currency_definitions: vec![],
            }
        }
    }
}

impl<Variants, First, Next, FilterMap, FindMap, CurrencyDefinitions>
    NonFinalizedSources<Variants, First, Next, FilterMap, FindMap, CurrencyDefinitions>
{
    #[inline]
    fn map_variants<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<R, First, Next, FilterMap, FindMap, CurrencyDefinitions>
    where
        F: FnOnce(Variants) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants: f(variants),
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_first<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, R, Next, FilterMap, FindMap, CurrencyDefinitions>
    where
        F: FnOnce(First) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first: f(first),
            next,
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_next<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, First, R, FilterMap, FindMap, CurrencyDefinitions>
    where
        F: FnOnce(Next) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            next: f(next),
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_filter_map<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, First, Next, R, FindMap, CurrencyDefinitions>
    where
        F: FnOnce(FilterMap) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            next,
            filter_map: f(filter_map),
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_find_map<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, First, Next, FilterMap, R, CurrencyDefinitions>
    where
        F: FnOnce(FindMap) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map: f(find_map),
            currency_definitions,
        }
    }

    #[inline]
    fn map_currency_definitions<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, First, Next, FilterMap, FindMap, R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<Variants, First, NextHead, NextMiddle, NextTail, FilterMap, FindMap, CurrencyDefinition>
    NonFinalizedSources<
        Vec<Variants>,
        Option<First>,
        Option<(NextHead, Vec<NextMiddle>, NextTail)>,
        Vec<FilterMap>,
        Vec<FindMap>,
        Vec<CurrencyDefinition>,
    >
{
    fn try_fold(
        self,
        element: GeneratedEntryResult<
            Variants,
            First,
            GroupMemberEntry<NextHead, NextMiddle, NextTail>,
            FilterMap,
            FindMap,
            CurrencyDefinition,
        >,
    ) -> Result<Self> {
        let Self {
            currencies_count,
            mut variants,
            first,
            next,
            mut filter_map,
            mut find_map,
            mut currency_definitions,
        } = self;

        currencies_count
            .checked_add(1)
            .context("Unable to increment generated currencies count! Overflowing beyond maximum capacity!")
            .and_then(|currencies_count| {
                element.map(|GeneratedEntry {
                    variant,
                    first_entry,
                    next_entry: GroupMemberEntry {
                        head: next_head_entry,
                        middle: next_middle_entry,
                        tail: next_tail_entry,
                    },
                    filter_map_entry,
                    find_map_entry,
                    currency_definition,
                 }| {
                    variants.push(variant);

                    let (next_head, next_middle) = if let Some((next_head, mut next_middle, _)) = next {
                        next_middle.push(next_middle_entry);

                        (next_head, next_middle)
                    } else {
                        (next_head_entry, vec![])
                    };

                    filter_map.push(filter_map_entry);

                    find_map.push(find_map_entry);

                    currency_definitions.push(currency_definition);

                    Self {
                        currencies_count,
                        variants,
                        first: first.or(Some(first_entry)),
                        next: Some((next_head, next_middle, next_tail_entry)),
                        filter_map,
                        find_map,
                        currency_definitions,
                    }
                })
            })
    }
}

impl<
    'r,
    'variants,
    'first,
    'next,
    'filter_map,
    'find_map,
    'currency_definition,
    Variants,
    First,
    Next,
    FilterMap,
    FindMap,
    CurrencyDefinitions,
>
    NonFinalizedSources<
        Variants,
        Option<First>,
        Option<Next>,
        Option<FilterMap>,
        Option<FindMap>,
        CurrencyDefinitions,
    >
where
    'variants: 'r,
    'first: 'r,
    'next: 'r,
    'filter_map: 'r,
    'find_map: 'r,
    'currency_definition: 'r,
    Variants: Iterator<Item = &'variants str>,
    First: Iterator<Item = &'first str>,
    Next: Iterator<Item = &'next str>,
    FilterMap: Iterator<Item = &'filter_map str>,
    FindMap: Iterator<Item = &'find_map str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
{
    #[inline]
    fn finalize(
        self,
    ) -> FinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
        + use<
            'r,
            'variants,
            'first,
            'next,
            'filter_map,
            'find_map,
            'currency_definition,
            Variants,
            First,
            Next,
            FilterMap,
            FindMap,
            CurrencyDefinitions,
        >,
    > {
        let Self {
            currencies_count,
            variants,
            first,
            next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        FinalizedSources {
            currencies_count,
            sources: iter::once(
                "// @generated

pub(super) enum GroupMember {",
            )
            .chain(variants.map(SubtypeLifetime::subtype))
            .chain(iter::once(
                "
}

impl currency::GroupMember<super::Group> for GroupMember {
    fn first() -> Option<Self> {
        ",
            ))
            .chain(first.map_or_else(
                || Either::Right(iter::once("None")),
                |first| Either::Left(first.map(SubtypeLifetime::subtype)),
            ))
            .chain(iter::once(
                "
    }

    fn next(&self) -> Option<Self> {
        match *self {",
            ))
            .chain(next.map_or(const { Either::Right(iter::empty()) }, |next| {
                Either::Left(next.map(SubtypeLifetime::subtype))
            }))
            .chain([
                "}
    }

    fn filter_map<FilterMap>(&self, ",
                if filter_map.is_some() {
                    "filter_map"
                } else {
                    "_"
                },
                ": &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: currency::GroupFilterMapT<VisitedG = super::Group>,
    {
        match *self {",
            ])
            .chain(
                filter_map.map_or(const { Either::Right(iter::empty()) }, |iter| {
                    Either::Left(iter.map(SubtypeLifetime::subtype))
                }),
            )
            .chain([
                "}
    }

    fn find_map<FindMap>(&self, ",
                if find_map.is_some() { "find_map" } else { "_" },
                ": FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: currency::GroupFindMapT<TargetG = super::Group>,
    {
        match *self {",
            ])
            .chain(
                find_map.map_or(const { Either::Right(iter::empty()) }, |iter| {
                    Either::Left(iter.map(SubtypeLifetime::subtype))
                }),
            )
            .chain(iter::once(
                "}
    }
}

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
