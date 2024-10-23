use std::{borrow::Cow, iter};

use anyhow::Result;

use crate::{either::Either, subtype_lifetime::SubtypeLifetime};

use super::{super::Generator, SourcesGenerator};

use self::currency_definition_generator::CurrencyDefinitionGenerator;

mod currency_definition_generator;

const NON_EXISTENT_DEX_CURRENCY: &str =
    "Queried ticker does not belong to any defined DEX currency!";

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, 'currencies_tree>
    SourcesGenerator<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'currencies_tree,
        '_,
        '_,
        '_,
        '_,
    >
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
        'dex_currencies: 'r,
        'ticker: 'r,
        Generator: self::Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
        Tickers: Iterator<Item = &'ticker str>,
    {
        if let Some(head_ticker) = tickers.next() {
            generate_non_empty_sources(
                CurrencyDefinitionGenerator {
                    dex_currencies: self.dex_currencies,
                    currencies_tree: self.currencies_tree,
                    generator,
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
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'parents_map,
    'parent,
    'children_map,
    'child,
    'generator,
    'ticker,
    Generator,
    Tickers,
>(
    currency_definition_generator: CurrencyDefinitionGenerator<
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'currencies_tree,
        'parents_map,
        'parent,
        'children_map,
        'child,
        'generator,
        Generator,
    >,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    NonFinalizedSources<
        impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, Generator, Tickers>,
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
    'ticker: 'r,
    'dex_currencies: 'r,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    Generator: self::Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
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

        let matcher_parameter = currency_definition_generator.matcher_parameter;
        let visitor_parameter = currency_definition_generator.visitor_parameter;
        let visit_function = currency_definition_generator.visit_function;

        iter::once(
            currency_definition_generator
                .generate_entry(head_ticker)
                .map(
                    |currency_definition_generator::Entry {
                         maybe_visit,
                         currency_definition,
                     }| {
                        (
                            Either::Left(maybe_visit.into_iter().map(SubtypeLifetime::subtype)),
                            currency_definition,
                        )
                    },
                ),
        )
        .chain(tail_tickers.map({
            |ticker| {
                currency_definition_generator.generate_entry(ticker).map(
                    |currency_definition_generator::Entry {
                         maybe_visit,
                         currency_definition,
                     }| {
                        (
                            Either::Right(
                                else_maybe_visit_entry(
                                    currency_definition_generator.visitor_parameter,
                                    maybe_visit,
                                )
                                .map(SubtypeLifetime::subtype),
                            ),
                            currency_definition,
                        )
                    },
                )
            }
        }))
        .try_fold(
            NonFinalizedSources::new(0, vec![], vec![]),
            |mut accumulator, element| {
                element.map(|(maybe_visit_entry, currency_definition)| {
                    accumulator.currencies_count += 1;

                    accumulator.maybe_visit.push(maybe_visit_entry);

                    accumulator.currency_definitions.push(currency_definition);

                    accumulator
                })
            },
        )
        .map(move |non_finalized_sources| {
            non_finalized_sources
                .map_maybe_visit(move |maybe_visit| {
                    finalize_non_empty_maybe_visit(
                        matcher_parameter,
                        visitor_parameter,
                        visit_function,
                        maybe_visit.into_iter().flatten(),
                    )
                })
                .map_currency_definitions(|currency_definitions| {
                    currency_definitions.into_iter().flatten()
                })
        })
    }
}

fn finalize_non_empty_maybe_visit<'r, MaybeVisit>(
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    visit_function: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str> + use<'r, MaybeVisit>
where
    MaybeVisit: IntoIterator<Item = &'r str>,
{
    finalize_maybe_visit(
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
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str> + use<'r, MaybeVisit>
where
    MaybeVisit: IntoIterator<Item = &'r str>,
{
    [
        r#"
pub(super) fn maybe_visit<M, V, VisitedG>(
    "#,
        matcher_parameter,
        r#": &M,
    "#,
        visitor_parameter,
        r#": V,
) -> currency::MaybeAnyVisitResult<VisitedG, V>
where
    super::Group: currency::MemberOf<VisitedG>,
    M: currency::Matcher,
    V: currency::AnyVisitor<VisitedG>,
    VisitedG: currency::Group<TopG = crate::payment::Group>,
{
    "#,
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
        finalize_maybe_visit("_", "visitor", iter::once("currency::visit_noone(visitor)")),
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
