use std::{borrow::Cow, iter};

use anyhow::Result;

use crate::{either::Either, subtype_lifetime::SubtypeLifetime};

use super::{
    super::{InPoolWithGenerator, MaybeVisitGenerator, PairsGroupGenerator, Resolver},
    SourcesGenerator,
};

use self::currency_definition_generator::CurrencyDefinitionGenerator;

mod currency_definition_generator;

impl<
        'host_currency,
        'dex_currencies,
        'definition,
        'dex_currency_ticker,
        'dex_currency_definition,
        'currencies_tree,
    > SourcesGenerator<'currencies_tree, '_, '_, '_, '_>
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
        'ticker: 'r,
        Generator: Resolver<'dex_currencies, 'definition>
            + MaybeVisitGenerator
            + PairsGroupGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
            + InPoolWithGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
        Tickers: Iterator<Item = &'ticker str>,
    {
        if let Some(head_ticker) = tickers.next() {
            generate_non_empty_sources(
                CurrencyDefinitionGenerator {
                    currencies_tree: self.currencies_tree,
                    generator,
                    visited_group: "VisitedG",
                    visit_function: "visit",
                    matcher_parameter: "matcher",
                    visitor_parameter: "visitor",
                },
                head_ticker,
                tickers,
            )
            .map(|non_finalized_sources| {
                non_finalized_sources
                    .map_maybe_visit(Either::Left)
                    .map_currency_definitions(Either::Left)
            })
        } else {
            Ok(empty_sources()
                .map_maybe_visit(Either::Right)
                .map_currency_definitions(Either::Right))
        }
        .map(NonFinalizedSources::finalize)
    }
}

fn generate_non_empty_sources<
    'r,
    'host_currency,
    'dex_currencies,
    'definition,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'generator,
    'ticker,
    Generator,
    Tickers,
>(
    currency_definition_generator: CurrencyDefinitionGenerator<
        'currencies_tree,
        '_,
        '_,
        '_,
        '_,
        'generator,
        Generator,
    >,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    NonFinalizedSources<
        impl Iterator<Item = &'dex_currencies str>
            + use<'dex_currencies, 'generator, Generator, Tickers>,
        impl Iterator<Item = Cow<'r, str>>
            + use<
                'r,
                'dex_currencies,
                'dex_currency_ticker,
                'dex_currency_definition,
                'currencies_tree,
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
    'ticker: 'r,
    Generator: Resolver<'dex_currencies, 'definition>
        + MaybeVisitGenerator
        + PairsGroupGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + InPoolWithGenerator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
    Tickers: Iterator<Item = &'ticker str>,
{
    {
        fn else_maybe_visit_entry<'r, MaybeVisitEntry>(
            visitor_parameter: &'static str,
            maybe_visit_entry: MaybeVisitEntry,
        ) -> impl Iterator<Item = &'r str> + use<'r, MaybeVisitEntry>
        where
            MaybeVisitEntry: IntoIterator<Item = &'r str>,
        {
            [
                "
        .or_else(|",
                visitor_parameter,
                "| ",
            ]
            .into_iter()
            .chain(maybe_visit_entry)
            .chain(iter::once(")"))
        }

        let visited_group = currency_definition_generator.visited_group;

        let matcher_parameter = currency_definition_generator.matcher_parameter;

        let visitor_parameter = currency_definition_generator.visitor_parameter;

        let visit_function = currency_definition_generator.visit_function;

        iter::once(
            currency_definition_generator
                .generate_entry(head_ticker)
                .map(|entry| {
                    (
                        entry
                            .maybe_visit
                            .map_left(IntoIterator::into_iter)
                            .map_left(Either::Left),
                        entry.currency_definition,
                    )
                }),
        )
        .chain(tail_tickers.map({
            |ticker| {
                currency_definition_generator
                    .generate_entry(ticker)
                    .map(|entry| {
                        (
                            match entry.maybe_visit {
                                Either::Left(iter) => Either::Left(Either::Right(
                                    else_maybe_visit_entry(visitor_parameter, iter),
                                )),
                                Either::Right(iter @ iter::Empty { .. }) => Either::Right(iter),
                            },
                            entry.currency_definition,
                        )
                    })
            }
        }))
        .try_fold(
            NonFinalizedSources::new(
                0,
                if <Generator as MaybeVisitGenerator>::GENERATE {
                    Either::Left(vec![])
                } else {
                    Either::Right(iter::empty())
                },
                vec![],
            ),
            |mut accumulator, element| {
                element.map(|(maybe_visit, currency_definition)| {
                    accumulator.currencies_count += 1;

                    match (&mut accumulator.maybe_visit, maybe_visit) {
                        (Either::Left(maybe_visit_iters), Either::Left(entry)) => {
                            maybe_visit_iters.push(entry)
                        }
                        (Either::Left { .. }, Either::Right { .. })
                        | (Either::Right { .. }, Either::Left { .. }) => unreachable!(),
                        (Either::Right(iter::Empty { .. }), Either::Right(iter::Empty { .. })) => {}
                    }

                    accumulator.currency_definitions.push(currency_definition);

                    accumulator
                })
            },
        )
        .map(move |non_finalized_sources| {
            non_finalized_sources
                .map_maybe_visit(move |maybe_visit| {
                    maybe_visit.map_left(|maybe_visit| {
                        finalize_non_empty_maybe_visit(
                            visited_group,
                            visit_function,
                            matcher_parameter,
                            visitor_parameter,
                            maybe_visit.into_iter().flatten(),
                        )
                    })
                })
                .map_currency_definitions(|currency_definitions| {
                    currency_definitions.into_iter().flatten()
                })
        })
    }
}

fn finalize_non_empty_maybe_visit<'r, MaybeVisit>(
    visited_group: &'static str,
    visit_function: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str> + use<'r, MaybeVisit>
where
    MaybeVisit: IntoIterator<Item = &'r str>,
{
    finalize_maybe_visit(
        visited_group,
        matcher_parameter,
        visitor_parameter,
        [
            "use currency::maybe_visit_member as ",
            visit_function,
            ";

    ",
        ]
        .into_iter()
        .chain(maybe_visit),
    )
}

fn finalize_maybe_visit<'r, MaybeVisit>(
    visited_group: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str> + use<'r, MaybeVisit>
where
    MaybeVisit: IntoIterator<Item = &'r str>,
{
    [
        "
pub(super) fn maybe_visit<M, V, ",
        visited_group,
        ">(
    ",
        matcher_parameter,
        ": &M,
    ",
        visitor_parameter,
        ": V,
) -> currency::MaybeAnyVisitResult<",
        visited_group,
        ", V>
where
    super::Group: currency::MemberOf<",
        visited_group,
        ">,
    M: currency::Matcher,
    V: currency::AnyVisitor<",
        visited_group,
        ">,
    ",
        visited_group,
        ": currency::Group<TopG = crate::payment::Group>,
{
    ",
    ]
    .into_iter()
    .chain(maybe_visit)
    .chain(iter::once(
        "
}
",
    ))
}

fn empty_sources<'maybe_visit, 'currency_definitions>() -> NonFinalizedSources<
    impl Iterator<Item = &'maybe_visit str> + use<'maybe_visit>,
    impl Iterator<Item = Cow<'currency_definitions, str>> + use<'currency_definitions>,
> {
    NonFinalizedSources::new(
        0,
        finalize_maybe_visit(
            "VisitedG",
            "_",
            "visitor",
            iter::once("currency::visit_noone(visitor)"),
        ),
        const { iter::empty() },
    )
}

struct NonFinalizedSources<MaybeVisit, CurrencyDefinitions> {
    currencies_count: usize,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

impl<MaybeVisit, CurrencyDefinitions> NonFinalizedSources<MaybeVisit, CurrencyDefinitions> {
    const fn new(
        currencies_count: usize,
        maybe_visit: MaybeVisit,
        currency_definitions: CurrencyDefinitions,
    ) -> Self {
        Self {
            currencies_count,
            maybe_visit,
            currency_definitions,
        }
    }

    fn map_maybe_visit<F, R>(self, f: F) -> NonFinalizedSources<R, CurrencyDefinitions>
    where
        F: FnOnce(MaybeVisit) -> R,
    {
        let Self {
            currencies_count,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            maybe_visit: f(maybe_visit),
            currency_definitions,
        }
    }

    fn map_currency_definitions<F, R>(self, f: F) -> NonFinalizedSources<MaybeVisit, R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            maybe_visit,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<'r, 'maybe_visit, 'currency_definition, MaybeVisit, CurrencyDefinitions>
    NonFinalizedSources<MaybeVisit, CurrencyDefinitions>
where
    'maybe_visit: 'r,
    'currency_definition: 'r,
    MaybeVisit: Iterator<Item = &'maybe_visit str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
{
    fn finalize(
        self,
    ) -> FinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
            + use<'r, 'maybe_visit, 'currency_definition, MaybeVisit, CurrencyDefinitions>,
    > {
        FinalizedSources {
            currencies_count: self.currencies_count,
            sources: iter::once("// @generated\n")
                .chain(self.maybe_visit.map(SubtypeLifetime::subtype))
                .map(Cow::Borrowed)
                .chain(self.currency_definitions.map(SubtypeLifetime::subtype)),
        }
    }
}

pub(super) struct FinalizedSources<Sources> {
    pub currencies_count: usize,
    pub sources: Sources,
}
