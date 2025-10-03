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
                    .map_variants(Either::Left)
                    .map_head_next(Either::Left)
                    .map_middle_next(Either::Left)
                    .map_tail_next(Either::Left)
                    .map_filter_map(Some)
                    .map_find_map(Some)
                    .map_currency_definitions(Either::Left)
            })
        } else {
            const {
                Ok(NonFinalizedSources {
                    currencies_count: 0,
                    variants: Either::Right(iter::empty()),
                    first: None,
                    head_next: Either::Right(iter::empty()),
                    middle_next: Either::Right(iter::empty()),
                    tail_next: Either::Right(iter::empty()),
                    filter_map: None,
                    find_map: None,
                    currency_definitions: Either::Right(iter::empty()),
                })
            }
        }
        .map(NonFinalizedSources::finalize)
    }
}

type WithCurrenciesSourcesResult<
    Variants,
    First,
    HeadNext,
    MiddleNext,
    TailNext,
    FilterMap,
    FindMap,
    CurrencyDefinitions,
> = Result<
    NonFinalizedSources<
        Variants,
        Option<First>,
        HeadNext,
        MiddleNext,
        TailNext,
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
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
    impl Iterator<Item = &'r str> + use<'r, Generator, Tickers>,
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
                .map_variants(flatten_iter_vec)
                .map_head_next(|head_next| {
                    head_next.map_or(const { Either::Right(iter::empty()) }, Either::Left)
                })
                .map_middle_next(flatten_iter_vec)
                .map_tail_next(|tail_next| {
                    tail_next.map_or(const { Either::Right(iter::empty()) }, Either::Left)
                })
                .map_filter_map(flatten_iter_vec)
                .map_find_map(flatten_iter_vec)
                .map_currency_definitions(flatten_iter_vec)
        })
}

struct NonFinalizedSources<
    Variants,
    First,
    HeadNext,
    MiddleNext,
    TailNext,
    FilterMap,
    FindMap,
    CurrencyDefinitions,
> {
    currencies_count: usize,
    variants: Variants,
    first: First,
    head_next: HeadNext,
    middle_next: MiddleNext,
    tail_next: TailNext,
    filter_map: FilterMap,
    find_map: FindMap,
    currency_definitions: CurrencyDefinitions,
}

impl<Variants, First, HeadNext, MiddleNext, TailNext, FilterMap, FindMap, CurrencyDefinitions>
    NonFinalizedSources<
        Vec<Variants>,
        Option<First>,
        Option<HeadNext>,
        Vec<MiddleNext>,
        Option<TailNext>,
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
                head_next: None,
                middle_next: vec![],
                tail_next: None,
                filter_map: vec![],
                find_map: vec![],
                currency_definitions: vec![],
            }
        }
    }
}

impl<Variants, First, HeadNext, MiddleNext, LastNext, FilterMap, FindMap, CurrencyDefinitions>
    NonFinalizedSources<
        Variants,
        First,
        HeadNext,
        MiddleNext,
        LastNext,
        FilterMap,
        FindMap,
        CurrencyDefinitions,
    >
{
    #[inline]
    fn map_variants<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        R,
        First,
        HeadNext,
        MiddleNext,
        LastNext,
        FilterMap,
        FindMap,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(Variants) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants: f(variants),
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_head_next<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        Variants,
        First,
        R,
        MiddleNext,
        LastNext,
        FilterMap,
        FindMap,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(HeadNext) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next: f(head_next),
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_middle_next<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        Variants,
        First,
        HeadNext,
        R,
        LastNext,
        FilterMap,
        FindMap,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(MiddleNext) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next: f(middle_next),
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_tail_next<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        Variants,
        First,
        HeadNext,
        MiddleNext,
        R,
        FilterMap,
        FindMap,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(LastNext) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next: f(tail_next),
            filter_map,
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_filter_map<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        Variants,
        First,
        HeadNext,
        MiddleNext,
        LastNext,
        R,
        FindMap,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(FilterMap) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map: f(filter_map),
            find_map,
            currency_definitions,
        }
    }

    #[inline]
    fn map_find_map<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<
        Variants,
        First,
        HeadNext,
        MiddleNext,
        LastNext,
        FilterMap,
        R,
        CurrencyDefinitions,
    >
    where
        F: FnOnce(FindMap) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map: f(find_map),
            currency_definitions,
        }
    }

    #[inline]
    fn map_currency_definitions<F, R>(
        self,
        f: F,
    ) -> NonFinalizedSources<Variants, First, HeadNext, MiddleNext, LastNext, FilterMap, FindMap, R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next,
            tail_next,
            filter_map,
            find_map,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<Variants, First, HeadNext, MiddleNext, TailNext, FilterMap, FindMap, CurrencyDefinition>
    NonFinalizedSources<
        Vec<Variants>,
        Option<First>,
        Option<HeadNext>,
        Vec<MiddleNext>,
        Option<TailNext>,
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
            HeadNext,
            MiddleNext,
            TailNext,
            FilterMap,
            FindMap,
            CurrencyDefinition,
        >,
    ) -> Result<Self> {
        let Self {
            currencies_count,
            mut variants,
            first,
            head_next,
            mut middle_next,
            tail_next: _,
            mut filter_map,
            mut find_map,
            mut currency_definitions,
        } = self;

        currencies_count
            .checked_add(1)
            .context("Unable to increment generated currencies count! Overflowing beyond maximum capacity!")
            .and_then(|currencies_count| {
                element.map(|GeneratedEntry { variant, first_entry, head_next_entry, middle_next_entry, tail_next_entry, filter_map_entry, find_map_entry, currency_definition }| {
                    variants.push(variant);

                    if first.is_some() {
                        middle_next.push(middle_next_entry);
                    }

                    filter_map.push(filter_map_entry);

                    find_map.push(find_map_entry);

                    currency_definitions.push(currency_definition);

                    Self {
                        currencies_count,
                        variants,
                        first: first.or(Some(first_entry)),
                        head_next: head_next.or(Some(head_next_entry)),
                        middle_next,
                        tail_next: Some(tail_next_entry),
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
    'head_next,
    'middle_next,
    'tail_next,
    'filter_map,
    'find_map,
    'currency_definition,
    Variants,
    First,
    HeadNext,
    MiddleNext,
    TailNext,
    FilterMap,
    FindMap,
    CurrencyDefinitions,
>
    NonFinalizedSources<
        Variants,
        Option<First>,
        HeadNext,
        MiddleNext,
        TailNext,
        Option<FilterMap>,
        Option<FindMap>,
        CurrencyDefinitions,
    >
where
    'variants: 'r,
    'first: 'r,
    'head_next: 'r,
    'middle_next: 'r,
    'tail_next: 'r,
    'filter_map: 'r,
    'find_map: 'r,
    'currency_definition: 'r,
    Variants: Iterator<Item = &'variants str>,
    First: Iterator<Item = &'first str>,
    HeadNext: Iterator<Item = &'head_next str>,
    MiddleNext: Iterator<Item = &'middle_next str>,
    TailNext: Iterator<Item = &'tail_next str>,
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
            'head_next,
            'middle_next,
            'tail_next,
            'filter_map,
            'find_map,
            'currency_definition,
            Variants,
            First,
            HeadNext,
            MiddleNext,
            TailNext,
            FilterMap,
            FindMap,
            CurrencyDefinitions,
        >,
    > {
        let Self {
            currencies_count,
            variants,
            first,
            head_next,
            middle_next: next,
            tail_next,
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
            .chain(head_next.map(SubtypeLifetime::subtype))
            .chain(next.map(SubtypeLifetime::subtype))
            .chain(tail_next.map(SubtypeLifetime::subtype))
            .chain([
                "}
    }

    fn filter_map<FilterMap>(&self, ",
                if filter_map.is_some() {
                    "filter_map"
                } else {
                    "_"
                },
                ": &FilterMap) -> Option<FilterMap::Outcome>    where
        FilterMap: currency::GroupFilterMap<VisitedG = super::Group>,
    {
        match *self {",
            ])
            .chain(
                filter_map.map_or(const { Either::Right(iter::empty()) }, |iter| {
                    Either::Left(iter.map(SubtypeLifetime::subtype).chain(iter::once(
                        "
        ",
                    )))
                }),
            )
            .chain([
                "}
    }

    fn find_map<FindMap>(&self, ",
                if find_map.is_some() { "find_map" } else { "_" },
                ": FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: currency::GroupFindMap<TargetG = super::Group>,
    {
        match *self {",
            ])
            .chain(
                find_map.map_or(const { Either::Right(iter::empty()) }, |iter| {
                    Either::Left(iter.map(SubtypeLifetime::subtype).chain(iter::once(
                        "
        ",
                    )))
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
