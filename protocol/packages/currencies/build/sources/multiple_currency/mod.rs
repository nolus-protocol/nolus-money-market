use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Write},
    iter,
    path::Path,
};

use anyhow::{Context as _, Result};

use topology::CurrencyDefinition;

use crate::{
    currencies_tree::CurrenciesTree, either::Either, protocol::Protocol,
    subtype_lifetime::SubtypeLifetime,
};

use super::{module_and_name::CurrentModule, DexCurrencies};

use self::currency_definition_generator::CurrencyDefinitionGenerator;

mod currency_definition_generator;

const NON_EXISTENT_DEX_CURRENCY: &str =
    "Queried ticker does not belong to any defined DEX currency!";

pub(super) struct SourcesGenerator<
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
> {
    protocol: &'protocol Protocol,
    host_currency: &'host_currency CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
    currencies_tree: &'currencies_tree CurrenciesTree<'parents_map, 'parent, 'children_map, 'child>,
}

impl<
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
    >
    SourcesGenerator<
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
    >
{
    pub const fn new(
        protocol: &'protocol Protocol,
        host_currency: &'host_currency CurrencyDefinition,
        dex_currencies: &'dex_currencies DexCurrencies<
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
        currencies_tree: &'currencies_tree CurrenciesTree<
            'parents_map,
            'parent,
            'children_map,
            'child,
        >,
    ) -> Self {
        Self {
            protocol,
            host_currency,
            dex_currencies,
            currencies_tree,
        }
    }
}

impl<'dex_currencies, 'currencies_tree>
    SourcesGenerator<'_, '_, 'dex_currencies, '_, '_, 'currencies_tree, '_, '_, '_, '_>
{
    pub fn generate_and_commit<'ticker, BuildReport, Tickers>(
        &self,
        build_report: BuildReport,
        output_file_path: &Path,
        current_module: CurrentModule,
        tickers: Tickers,
    ) -> Result<()>
    where
        BuildReport: Write,
        Tickers: IntoIterator<Item = &'ticker str>,
    {
        self.generate_sources(current_module, tickers.into_iter())
            .and_then(|sources| create_file_and_commit(build_report, output_file_path, sources))
    }

    fn generate_sources<'r, 'ticker, Tickers>(
        &self,
        current_module: CurrentModule,
        mut tickers: Tickers,
    ) -> Result<
        FinalizedSources<impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree, Tickers>>,
    >
    where
        'dex_currencies: 'r,
        'ticker: 'r,
        Tickers: Iterator<Item = &'ticker str>,
    {
        if let Some(head_ticker) = tickers.next() {
            generate_non_empty_sources(
                self.dex_currencies,
                CurrencyDefinitionGenerator::new(
                    current_module,
                    self.protocol,
                    self.host_currency,
                    self.dex_currencies,
                    self.currencies_tree,
                ),
                "visit",
                "matcher",
                "visitor",
                head_ticker,
                tickers,
            )
            .map(NonFinalizedSources::wrap_either_left)
        } else {
            Ok(NonFinalizedSources {
                currencies_count: 0,
                matcher_parameter: "_",
                visitor_parameter: "visitor",
                maybe_visit: Either::Right(iter::once("currency::visit_noone(visitor)")),
                currency_definitions: Either::Right(iter::once(const { Cow::Borrowed("") })),
            })
        }
        .map(NonFinalizedSources::finalize)
    }
}

fn create_file_and_commit<'r, BuildReport, Sources>(
    build_report: BuildReport,
    output_file_path: &Path,
    sources: FinalizedSources<Sources>,
) -> Result<()>
where
    BuildReport: Write,
    Sources: Iterator<Item = Cow<'r, str>>,
{
    File::create(output_file_path)
        .map(BufWriter::new)
        .context("Failed to open output file for writing!")
        .and_then(|output_file| {
            commit_sources(output_file_path, output_file, sources, build_report)
        })
}

fn commit_sources<'r, Sources, Output, BuildReport>(
    output_file_path: &Path,
    mut output_file: Output,
    sources: FinalizedSources<Sources>,
    mut build_report: BuildReport,
) -> Result<()>
where
    Output: Write,
    BuildReport: Write,
    Sources: Iterator<Item = Cow<'r, str>>,
{
    let FinalizedSources {
        currencies_count,
        mut sources,
    } = sources;

    sources
        .try_for_each(|segment| output_file.write_all(segment.as_bytes()))
        .and_then(|()| output_file.flush())
        .with_context(|| {
            format!("Failed to write generated sources for output file {output_file_path:?}!")
        })
        .and_then(|()| {
            build_report
                .write_fmt(format_args!(
                    "{output_file_path:?}: {currencies_count} currencies emitted.\n",
                ))
                .context("Failed to write build report!")
        })
}

fn generate_non_empty_sources<'r, 'dex_currencies, 'currencies_tree, 'ticker, Tickers>(
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    currency_definition_generator: CurrencyDefinitionGenerator<
        '_,
        '_,
        'dex_currencies,
        '_,
        '_,
        'currencies_tree,
        '_,
        '_,
        '_,
        '_,
    >,
    visit_function: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    NonFinalizedSources<
        impl Iterator<Item = &'r str> + use<'r, 'currencies_tree, Tickers>,
        impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree, Tickers>,
    >,
>
where
    'ticker: 'r,
    'dex_currencies: 'r,
    Tickers: Iterator<Item = &'ticker str>,
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

    let process_ticker = move |ticker| {
        process_ticker(
            dex_currencies,
            &currency_definition_generator,
            visit_function,
            matcher_parameter,
            visitor_parameter,
            ticker,
        )
    };

    iter::once(
        process_ticker(head_ticker).map(|(maybe_visit_entry, currency_definition)| {
            (
                Either::Left(maybe_visit_entry.into_iter()),
                currency_definition,
            )
        }),
    )
    .chain(tail_tickers.map({
        move |ticker| {
            process_ticker(ticker).map(|(maybe_visit_entry, currency_definition)| {
                (
                    Either::Right(else_maybe_visit_entry(visitor_parameter, maybe_visit_entry)),
                    currency_definition,
                )
            })
        }
    }))
    .collect::<Result<_, _>>()
    .map(|sources| postprocess_sources_iterators(visit_function, sources))
    .map(
        |PostprocessedSources {
             currencies_count,
             maybe_visit,
             currency_definitions,
         }| {
            NonFinalizedSources::new(
                currencies_count,
                matcher_parameter,
                visitor_parameter,
                maybe_visit,
                currency_definitions,
            )
        },
    )
}

fn postprocess_sources_iterators<
    'maybe_visit,
    'currency_definition,
    MaybeVisit: Iterator<Item = &'maybe_visit str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
>(
    visit_function: &'static str,
    sources: Vec<(MaybeVisit, CurrencyDefinitions)>,
) -> PostprocessedSources<
    impl Iterator<Item = &'maybe_visit str> + use<'maybe_visit, MaybeVisit, CurrencyDefinitions>,
    impl Iterator<Item = Cow<'currency_definition, str>>
        + use<'currency_definition, MaybeVisit, CurrencyDefinitions>,
> {
    fn maybe_visit_prepend<'r>(
        visit_function: &'static str,
    ) -> impl Iterator<Item = &'r str> + use<'r> {
        [
            "use currency::maybe_visit_member as ",
            visit_function,
            ";

    ",
        ]
        .into_iter()
    }

    const CURRENCY_DEFINITIONS_PREPEND: Cow<'_, str> = Cow::Borrowed(
        "
pub(crate) mod definitions {",
    );

    const CURRENCY_DEFINITIONS_APPEND: Cow<'_, str> = Cow::Borrowed(
        "}
",
    );

    let currencies_count = sources.len();

    let (maybe_visit, currency_definitions): (Vec<_>, Vec<_>) = sources.into_iter().unzip();

    PostprocessedSources {
        currencies_count,
        maybe_visit: maybe_visit_prepend(visit_function).chain(maybe_visit.into_iter().flatten()),
        currency_definitions: iter::once(CURRENCY_DEFINITIONS_PREPEND)
            .chain(currency_definitions.into_iter().flatten())
            .chain(iter::once(CURRENCY_DEFINITIONS_APPEND)),
    }
}

struct PostprocessedSources<MaybeVisit, CurrencyDefinitions> {
    currencies_count: usize,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

fn process_ticker<'r, 'dex_currencies, 'currencies_tree>(
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    currency_definition_generator: &CurrencyDefinitionGenerator<
        '_,
        '_,
        'dex_currencies,
        '_,
        '_,
        'currencies_tree,
        '_,
        '_,
        '_,
        '_,
    >,
    visit_function: &'static str,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    ticker: &'r str,
) -> Result<(
    impl IntoIterator<Item = &'r str> + use<'r>,
    impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree>,
)>
where
    'dex_currencies: 'r,
{
    dex_currencies
        .get(ticker)
        .context(NON_EXISTENT_DEX_CURRENCY)
        .map(|(name, _)| {
            [
                visit_function,
                "::<_, self::definitions::",
                name,
                ", VisitedG, _>(",
                matcher_parameter,
                ", ",
                visitor_parameter,
                ")",
            ]
        })
        .and_then(|maybe_visit_entry| {
            currency_definition_generator
                .generate_entry(ticker)
                .map(|currency_definition| (maybe_visit_entry, currency_definition))
        })
}

struct NonFinalizedSources<MaybeVisit, CurrencyDefinitions> {
    currencies_count: usize,
    matcher_parameter: &'static str,
    visitor_parameter: &'static str,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

impl<MaybeVisit, CurrencyDefinitions> NonFinalizedSources<MaybeVisit, CurrencyDefinitions> {
    const fn new(
        currencies_count: usize,
        matcher_parameter: &'static str,
        visitor_parameter: &'static str,
        maybe_visit: MaybeVisit,
        currency_definitions: CurrencyDefinitions,
    ) -> Self {
        Self {
            currencies_count,
            matcher_parameter,
            visitor_parameter,
            maybe_visit,
            currency_definitions,
        }
    }

    fn wrap_either_left<MaybeVisitRight, CurrencyDefinitionsRight>(
        self,
    ) -> NonFinalizedSources<
        Either<MaybeVisit, MaybeVisitRight>,
        Either<CurrencyDefinitions, CurrencyDefinitionsRight>,
    > {
        let Self {
            currencies_count,
            matcher_parameter,
            visitor_parameter,
            maybe_visit,
            currency_definitions,
        } = self;

        NonFinalizedSources {
            currencies_count,
            matcher_parameter,
            visitor_parameter,
            maybe_visit: Either::Left(maybe_visit),
            currency_definitions: Either::Left(currency_definitions),
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
            sources: [
                r#"// @generated

pub(super) fn maybe_visit<M, V, VisitedG>(
    "#,
                self.matcher_parameter,
                r#": &M,
    "#,
                self.visitor_parameter,
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
            .chain(self.maybe_visit.map(SubtypeLifetime::subtype))
            .chain(iter::once(
                "
}
",
            ))
            .map(Cow::Borrowed)
            .chain(self.currency_definitions.map(SubtypeLifetime::subtype)),
        }
    }
}

struct FinalizedSources<Sources> {
    currencies_count: usize,
    sources: Sources,
}
