use std::{borrow::Cow, iter};

use anyhow::Result;
use either::Either;

use crate::subtype_lifetime::SubtypeLifetime;

use super::{super::generator, FinalizedSources, Writer};

use self::currency_definition::CurrencyDefinition;

mod currency_definition;

type TryFoldElement<'maybe_visit, Currencies, MaybeVisit, CurrencyDefinition> = Result<(
    Currencies,
    Either<MaybeVisit, iter::Empty<&'maybe_visit str>>,
    CurrencyDefinition,
)>;

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, 'currencies_tree>
    Writer<'currencies_tree, '_, '_, '_, '_>
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
        Generator: generator::Resolver<'dex_currencies, 'dex_currencies>
            + generator::MaybeVisit
            + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
            + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
        Tickers: Iterator<Item = &'ticker str>,
    {
        if let Some(head_ticker) = tickers.next() {
            with_currencies_sources(
                CurrencyDefinition::new(
                    self.currencies_tree,
                    generator,
                    "VisitedG",
                    "visit",
                    "matcher",
                    "visitor",
                ),
                head_ticker,
                tickers,
            )
            .map(|non_finalized_sources| {
                non_finalized_sources
                    .map_currencies(Either::Left)
                    .map_maybe_visit(Either::Left)
                    .map_currency_definitions(Either::Left)
            })
        } else {
            Ok(generate_blank_sources()
                .map_currencies(Either::Right)
                .map_maybe_visit(Either::Right)
                .map_currency_definitions(Either::Right))
        }
        .map(NonFinalizedSources::finalize)
    }
}

fn with_currencies_sources<
    'r,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'generator,
    'ticker,
    Generator,
    Tickers,
>(
    generator: CurrencyDefinition<'currencies_tree, '_, '_, '_, '_, 'generator, Generator>,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    NonFinalizedSources<
        impl Iterator<Item = &'dex_currencies str>
        + use<'dex_currencies, 'generator, Generator, Tickers>,
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
    'dex_currencies: 'r,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    'ticker: 'r,
    Generator: generator::Resolver<'dex_currencies, 'dex_currencies>
        + generator::MaybeVisit
        + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
    Tickers: Iterator<Item = &'ticker str>,
{
    let visited_group = generator.visited_group();

    let visit_function = generator.visit_function();

    let matcher_parameter = generator.matcher_parameter();

    let visitor_parameter = generator.visitor_parameter();

    per_currency_sources(generator, head_ticker, tail_tickers).map(|non_finalized_sources| {
        non_finalized_sources
            .map_currencies(currencies_definition)
            .map_maybe_visit(|maybe_visit| {
                maybe_visit.map_left(|maybe_visit| {
                    maybe_visit_definition_with_currencies(
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

struct GeneratedSourceEntry<Currencies, MaybeVisit, CurrencyDefinition> {
    currencies: Currencies,
    maybe_visit: MaybeVisit,
    currency_definition: CurrencyDefinition,
}

type PerCurrencySourcesResult<'dex_currencies, Currencies, MaybeVisit, CurrencyDefinition> = Result<
    NonFinalizedSources<
        Vec<Currencies>,
        Either<Vec<MaybeVisit>, iter::Empty<&'dex_currencies str>>,
        Vec<CurrencyDefinition>,
    >,
>;

fn per_currency_sources<
    'r,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'generator,
    'ticker,
    Generator,
    Tickers,
>(
    generator: CurrencyDefinition<'currencies_tree, '_, '_, '_, '_, 'generator, Generator>,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> PerCurrencySourcesResult<
    'dex_currencies,
    impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, 'generator, Generator, Tickers>,
    impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, 'generator, Generator, Tickers>,
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
>
where
    'dex_currencies: 'r,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    'ticker: 'r,
    Generator: generator::Resolver<'dex_currencies, 'dex_currencies>
        + generator::MaybeVisit
        + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
    Tickers: Iterator<Item = &'ticker str>,
{
    iter::once(generator.generate_entry(head_ticker).map(|entry| {
        (
            entry.currencies,
            entry
                .maybe_visit
                .map_left(|maybe_visit| Either::Left(maybe_visit.into_iter())),
            entry.currency_definition,
        )
    }))
    .chain(tail_tickers.map(|ticker| {
        generator.generate_entry(ticker).map(|entry| {
            (
                entry.currencies,
                entry.maybe_visit.map_left(|maybe_visit| {
                    Either::Right({
                        [
                            "
        .or_else(|",
                            generator.visitor_parameter(),
                            "| ",
                        ]
                        .into_iter()
                        .chain(maybe_visit)
                        .chain(iter::once(")"))
                    })
                }),
                entry.currency_definition,
            )
        })
    }))
    .try_fold(
        NonFinalizedSources::new(
            0,
            vec![],
            if <Generator as generator::MaybeVisit>::GENERATE {
                Either::Left(vec![])
            } else {
                Either::Right(iter::empty())
            },
            vec![],
        ),
        NonFinalizedSources::try_fold,
    )
}

#[inline]
fn currencies_definition<'r, Currencies>(currencies: Currencies) -> impl Iterator<Item = &'r str>
where
    Currencies: IntoIterator,
    Currencies::Item: Iterator<Item = &'r str>,
{
    iter::once(
        "
pub(super) fn currencies() -> impl Iterator<Item = currency::CurrencyDTO<super::Group>> {
    [",
    )
    .chain(currencies.into_iter().flat_map(|currencies| {
        iter::once("\n        ")
            .chain(currencies)
            .chain(iter::once(",\n    "))
    }))
    .chain(iter::once(
        "]
    .into_iter()
}
",
    ))
}

#[inline]
fn maybe_visit_definition_with_currencies<'r, MaybeVisit>(
    visited_group: &'static str,
    visit_function: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str>
where
    MaybeVisit: IntoIterator<Item = &'r str>,
{
    maybe_visit_definition(
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

#[inline]
fn maybe_visit_definition<'r, MaybeVisit>(
    visited_group: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
) -> impl Iterator<Item = &'r str>
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

#[inline]
fn generate_blank_sources<'currencies, 'maybe_visit, 'currency_definitions>() -> NonFinalizedSources<
    impl Iterator<Item = &'currencies str> + use<'currencies>,
    impl Iterator<Item = &'maybe_visit str> + use<'maybe_visit>,
    impl Iterator<Item = Cow<'currency_definitions, str>> + use<'currency_definitions>,
> {
    const VISITOR_PARAMETER: &str = "visitor";

    NonFinalizedSources::new(
        0,
        const { iter::empty() },
        maybe_visit_definition(
            "VisitedG",
            "_",
            VISITOR_PARAMETER,
            ["currency::visit_noone(", VISITOR_PARAMETER, ")"],
        ),
        const { iter::empty() },
    )
}

struct NonFinalizedSources<Currencies, MaybeVisit, CurrencyDefinitions> {
    currencies_count: usize,
    currencies: Currencies,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

impl<Currencies, MaybeVisit, CurrencyDefinitions>
    NonFinalizedSources<Currencies, MaybeVisit, CurrencyDefinitions>
{
    #[inline]
    const fn new(
        currencies_count: usize,
        currencies: Currencies,
        maybe_visit: MaybeVisit,
        currency_definitions: CurrencyDefinitions,
    ) -> Self {
        Self {
            currencies_count,
            currencies,
            maybe_visit,
            currency_definitions,
        }
    }

    #[inline]
    fn map_currencies<F, R>(self, f: F) -> NonFinalizedSources<R, MaybeVisit, CurrencyDefinitions>
    where
        F: FnOnce(Currencies) -> R,
    {
        let Self {
            currencies_count,
            currencies,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currencies: f(currencies),
            maybe_visit,
            currency_definitions,
        }
    }

    #[inline]
    fn map_maybe_visit<F, R>(self, f: F) -> NonFinalizedSources<Currencies, R, CurrencyDefinitions>
    where
        F: FnOnce(MaybeVisit) -> R,
    {
        let Self {
            currencies_count,
            currencies,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currencies,
            maybe_visit: f(maybe_visit),
            currency_definitions,
        }
    }

    #[inline]
    fn map_currency_definitions<F, R>(self, f: F) -> NonFinalizedSources<Currencies, MaybeVisit, R>
    where
        F: FnOnce(CurrencyDefinitions) -> R,
    {
        let Self {
            currencies_count,
            currencies,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            currencies,
            maybe_visit,
            currency_definitions: f(currency_definitions),
        }
    }
}

impl<'maybe_visit, Currencies, MaybeVisit, CurrencyDefinition>
    NonFinalizedSources<
        Vec<Currencies>,
        Either<Vec<MaybeVisit>, iter::Empty<&'maybe_visit str>>,
        Vec<CurrencyDefinition>,
    >
{
    fn try_fold(
        Self {
            currencies_count,
            mut currencies,
            maybe_visit,
            mut currency_definitions,
        }: Self,
        element: TryFoldElement<'maybe_visit, Currencies, MaybeVisit, CurrencyDefinition>,
    ) -> Result<Self> {
        element.map(
            |(currencies_entry, maybe_visit_entry, currency_definition)| NonFinalizedSources {
                currencies_count: currencies_count + 1,
                currencies: {
                    currencies.push(currencies_entry);

                    currencies
                },
                maybe_visit: match (maybe_visit, maybe_visit_entry) {
                    (Either::Left(mut maybe_visit), Either::Left(entry)) => {
                        maybe_visit.push(entry);

                        Either::Left(maybe_visit)
                    }
                    (Either::Left { .. }, Either::Right { .. })
                    | (Either::Right { .. }, Either::Left { .. }) => unreachable!(),
                    (
                        iter @ Either::Right(iter::Empty { .. }),
                        Either::Right(iter::Empty { .. }),
                    ) => iter,
                },
                currency_definitions: {
                    currency_definitions.push(currency_definition);

                    currency_definitions
                },
            },
        )
    }
}

impl<
    'r,
    'currencies,
    'maybe_visit,
    'currency_definition,
    Currencies,
    MaybeVisit,
    CurrencyDefinitions,
> NonFinalizedSources<Currencies, MaybeVisit, CurrencyDefinitions>
where
    'currencies: 'r,
    'maybe_visit: 'r,
    'currency_definition: 'r,
    Currencies: Iterator<Item = &'currencies str>,
    MaybeVisit: Iterator<Item = &'maybe_visit str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
{
    #[inline]
    fn finalize(
        self,
    ) -> FinalizedSources<
        impl Iterator<Item = Cow<'r, str>>
        + use<
            'r,
            'currencies,
            'maybe_visit,
            'currency_definition,
            Currencies,
            MaybeVisit,
            CurrencyDefinitions,
        >,
    > {
        FinalizedSources {
            currencies_count: self.currencies_count,
            sources: iter::once("// @generated\n")
                .chain(self.currencies.map(SubtypeLifetime::subtype))
                .chain(self.maybe_visit.map(SubtypeLifetime::subtype))
                .chain(iter::once(
                    r#"
pub(super) mod definitions {"#,
                ))
                .map(Cow::Borrowed)
                .chain(self.currency_definitions.map(SubtypeLifetime::subtype))
                .chain(iter::once(
                    const {
                        Cow::Borrowed(
                            r#"}
"#,
                        )
                    },
                )),
        }
    }
}
