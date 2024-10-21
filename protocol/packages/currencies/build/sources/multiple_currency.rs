use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    iter,
    path::Path,
};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{
    currencies_tree::CurrenciesTree, either::Either, protocol::Protocol,
    subtype_lifetime::SubtypeLifetime,
};

use super::module_and_name::{CurrentModule, ModuleAndName};

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
    pub protocol: &'protocol Protocol,
    pub host_currency: &'host_currency CurrencyDefinition,
    pub dex_currencies:
        &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
    pub currencies_tree:
        &'currencies_tree CurrenciesTree<'parents_map, 'parent, 'children_map, 'child>,
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
            .and_then(|sources| {
                File::create(output_file_path)
                    .map(BufWriter::new)
                    .context("Failed to open output file for writing!")
                    .and_then(|output_file| {
                        commit_sources(output_file_path, output_file, sources, build_report)
                    })
            })
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
        if let Some(ticker) = tickers.next() {
            generate_non_empty_sources(
                self.dex_currencies,
                CurrencyDefinitionGenerator {
                    current_module,
                    protocol: self.protocol,
                    host_currency: self.host_currency,
                    dex_currencies: self.dex_currencies,
                    currencies_tree: self.currencies_tree,
                },
                ticker,
                tickers,
            )
        } else {
            Ok(GeneratedSources {
                currencies_count: 0,
                matcher_parameter_name: "_",
                maybe_visit: Either::Right(iter::once("currency::visit_noone(visitor)")),
                currency_definitions: Either::Right(iter::once(const { Cow::Borrowed("") })),
            })
        }
        .map(GeneratedSources::finalize)
    }
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

fn generate_non_empty_sources<
    'r,
    'dex_currencies,
    'currencies_tree,
    'ticker,
    Tickers,
    MaybeVisitRight,
    CurrencyDefinitionsRight,
>(
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
    head_ticker: &'ticker str,
    tail_tickers: Tickers,
) -> Result<
    GeneratedSources<
        Either<
            impl Iterator<Item = &'r str>
                + use<'r, 'currencies_tree, Tickers, MaybeVisitRight, CurrencyDefinitionsRight>,
            MaybeVisitRight,
        >,
        Either<
            impl Iterator<Item = Cow<'r, str>>
                + use<'r, 'currencies_tree, Tickers, MaybeVisitRight, CurrencyDefinitionsRight>,
            CurrencyDefinitionsRight,
        >,
    >,
>
where
    'ticker: 'r,
    'dex_currencies: 'r,
    Tickers: Iterator<Item = &'ticker str>,
{
    iter::once(
        process_ticker(dex_currencies, &currency_definition_generator, head_ticker).map(
            |(maybe_visit_entry, currency_definition)| {
                (
                    Either::Left(maybe_visit_entry.into_iter()),
                    currency_definition,
                )
            },
        ),
    )
    .chain(tail_tickers.map({
        |ticker| {
            process_ticker(dex_currencies, &currency_definition_generator, ticker).map(
                |(maybe_visit_entry, currency_definition)| {
                    (
                        Either::Right(
                            iter::once(
                                "
                            .or_else(|visitor| ",
                            )
                            .chain(maybe_visit_entry)
                            .chain(iter::once(")")),
                        ),
                        currency_definition,
                    )
                },
            )
        }
    }))
    .collect::<Result<_, _>>()
    .map(postprocess_sources_iterators)
    .map(
        |(currencies_count, maybe_visit_body, currency_definitions)| GeneratedSources {
            currencies_count,
            matcher_parameter_name: "matcher",
            maybe_visit: Either::Left(maybe_visit_body),
            currency_definitions: Either::Left(currency_definitions),
        },
    )
}

fn process_ticker<'r, 'dex_currencies, 'currencies_tree, 'ticker>(
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
    ticker: &'ticker str,
) -> Result<(
    impl IntoIterator<Item = &'dex_currencies str> + use<'dex_currencies>,
    impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree>,
)>
where
    'ticker: 'r,
    'dex_currencies: 'r,
{
    maybe_visit_entry(dex_currencies, ticker).and_then(|maybe_visit_entry| {
        currency_definition_generator
            .generate(ticker)
            .map(|currency_definition| (maybe_visit_entry, currency_definition))
    })
}

fn postprocess_sources_iterators<
    'maybe_visit,
    'currency_definition,
    MaybeVisit: Iterator<Item = &'maybe_visit str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definition, str>>,
>(
    sources: Vec<(MaybeVisit, CurrencyDefinitions)>,
) -> (
    usize,
    impl Iterator<Item = &'maybe_visit str> + use<'maybe_visit, MaybeVisit, CurrencyDefinitions>,
    impl Iterator<Item = Cow<'currency_definition, str>>
        + use<'currency_definition, MaybeVisit, CurrencyDefinitions>,
) {
    const MAYBE_VISIT_BODY_PREPEND: &str = "use currency::maybe_visit_member as visit;

    ";

    const CURRENCY_DEFINITIONS_PREPEND: Cow<'_, str> = Cow::Borrowed(
        "
pub(crate) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
        PairsVisitor,
    };
    use sdk::schemars::JsonSchema;

    use crate::payment;
",
    );

    const CURRENCY_DEFINITIONS_APPEND: Cow<'_, str> = Cow::Borrowed(
        "}
",
    );

    let currencies_count = sources.len();

    let (maybe_visit, currency_definitions): (Vec<_>, Vec<_>) = sources.into_iter().unzip();

    (
        currencies_count,
        iter::once(MAYBE_VISIT_BODY_PREPEND).chain(maybe_visit.into_iter().flatten()),
        iter::once(CURRENCY_DEFINITIONS_PREPEND)
            .chain(currency_definitions.into_iter().flatten())
            .chain(iter::once(CURRENCY_DEFINITIONS_APPEND)),
    )
}

type DexCurrencies<'ticker, 'currency_definition> =
    BTreeMap<&'ticker str, (String, &'currency_definition CurrencyDefinition)>;

fn maybe_visit_entry<'dex_currencies>(
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    ticker: &str,
) -> Result<impl IntoIterator<Item = &'dex_currencies str> + use<'dex_currencies>> {
    dex_currencies
        .get(ticker)
        .context(NON_EXISTENT_DEX_CURRENCY)
        .map(|(name, _)| {
            [
                "visit::<_, self::definitions::",
                name,
                ", VisitedG, _>(matcher, visitor)",
            ]
        })
}

struct GeneratedSources<MaybeVisit, CurrencyDefinitions> {
    currencies_count: usize,
    matcher_parameter_name: &'static str,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

impl<'r, 'maybe_visit, 'currency_definition, MaybeVisit, CurrencyDefinitions>
    GeneratedSources<MaybeVisit, CurrencyDefinitions>
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
            sources: Self::finalize_sources(
                self.matcher_parameter_name,
                self.maybe_visit,
                self.currency_definitions,
            ),
        }
    }

    fn finalize_sources(
        matcher_parameter_name: &'static str,
        maybe_visit: MaybeVisit,
        currency_definitions: CurrencyDefinitions,
    ) -> impl Iterator<Item = Cow<'r, str>>
           + use<'r, 'maybe_visit, 'currency_definition, MaybeVisit, CurrencyDefinitions> {
        [
            r#"// @generated

use currency::{{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf}};

use crate::payment;

pub(super) fn maybe_visit<M, V, VisitedG>(
    "#,
            matcher_parameter_name,
            r#": &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    super::Group: MemberOf<VisitedG>,
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = payment::Group>,
{
    "#,
        ]
        .into_iter()
        .chain(maybe_visit.map(SubtypeLifetime::subtype))
        .chain(iter::once(
            "
}
",
        ))
        .map(Cow::Borrowed)
        .chain(currency_definitions.map(SubtypeLifetime::subtype))
    }
}

struct FinalizedSources<Sources> {
    currencies_count: usize,
    sources: Sources,
}

struct CurrencyDefinitionGenerator<
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
    current_module: CurrentModule,
    protocol: &'protocol Protocol,
    host_currency: &'host_currency CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
    currencies_tree: &'currencies_tree CurrenciesTree<'parents_map, 'parent, 'children_map, 'child>,
}

impl<'dex_currencies, 'currencies_tree>
    CurrencyDefinitionGenerator<'_, '_, 'dex_currencies, '_, '_, 'currencies_tree, '_, '_, '_, '_>
{
    fn generate<'r, 'ticker>(
        &self,
        ticker: &'ticker str,
    ) -> Result<impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree>>
    where
        'dex_currencies: 'r,
        'ticker: 'r,
    {
        let parents = self.currencies_tree.parents(ticker);

        let children = self.currencies_tree.children(ticker);

        [children, parents].into_iter().try_for_each({
            |paired_with| {
                if paired_with.contains(ticker) {
                    Err(anyhow!("Currency cannot be in a pool with itself!"))
                } else {
                    Ok(())
                }
            }
        })?;

        let (name, currency) = self
            .dex_currencies
            .get(ticker)
            .context(NON_EXISTENT_DEX_CURRENCY)?;

        let pairs_group = pairs_group(
            self.current_module,
            self.protocol,
            self.host_currency,
            self.dex_currencies,
            children.iter().copied(),
        )?;

        let in_pool_with = in_pool_with(
            self.current_module,
            self.protocol,
            self.host_currency,
            self.dex_currencies,
            parents.iter().copied(),
            name,
        )?;

        Ok([
            r#"
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    #[schemars(crate = "sdk::schemars")]
    pub struct "#,
            name,
            r#"(CurrencyDTO<super::super::Group>);

    impl CurrencyDef for "#,
            name,
            r#" {
        type Group = super::super::Group;

        fn definition() -> &'static Self {
            const {
                &Self(CurrencyDTO::new(
                    const {
                        &Definition::new(
                            ""#,
            ticker,
            r#"",
                            // "#,
            currency.host().path(),
            r#"
                            ""#,
            currency.host().symbol(),
            r#"",
                            // "#,
            currency.dex().path(),
            r#"
                            ""#,
            currency.dex().symbol(),
            r#"",
                            "#,
        ]
        .into_iter()
        .map(Cow::Borrowed)
        .chain(iter::once(Cow::Owned(
            currency.decimal_digits().to_string(),
        )))
        .chain(
            [
                r#",
                        )
                    },
                ))
            }
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    impl PairsGroup for "#,
                name,
                r#" {
        type CommonGroup = payment::Group;

        fn maybe_visit<M, V>("#,
                pairs_group.matcher_parameter_name,
                r#": &M, visitor: V) -> MaybePairsVisitorResult<V>
        where
            M: Matcher,
            V: PairsVisitor<Pivot = Self>,
        {
            "#,
            ]
            .into_iter()
            .map(Cow::Borrowed),
        )
        .chain(pairs_group.sources.map(Cow::Borrowed))
        .chain(iter::once(
            const {
                Cow::Borrowed(
                    r#"
        }
    }
"#,
                )
            },
        ))
        .chain(in_pool_with.map(Cow::Borrowed)))
    }
}

fn pairs_group<'dex_currencies, 'child, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    mut children: Children,
) -> Result<PairsGroup<impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, Children>>>
where
    Children: Iterator<Item = &'child str>,
{
    const PAIRS_GROUP_ENTRIES_PREPEND: &str = "use currency::maybe_visit_buddy as visit;

            ";

    fn pairs_group_entry<'dex_currencies>(
        current_module: CurrentModule,
        protocol: &Protocol,
        host_currency: &CurrencyDefinition,
        dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
        ticker: &str,
    ) -> Result<impl IntoIterator<Item = &'dex_currencies str> + use<'dex_currencies>> {
        ModuleAndName::resolve(
            current_module,
            protocol,
            host_currency,
            dex_currencies,
            ticker,
        )
        .map(|resolved| {
            [
                "visit::<",
                resolved.module(),
                "::",
                resolved.name(),
                ", _, _>(matcher, visitor)",
            ]
        })
    }

    if let Some(ticker) = children.next() {
        let process_ticker = |ticker: &str| {
            pairs_group_entry(
                current_module,
                protocol,
                host_currency,
                dex_currencies,
                ticker,
            )
        };

        process_ticker(ticker)
            .and_then(|first_entry| {
                children
                    .map(|ticker| {
                        process_ticker(ticker).map(|processed| {
                            iter::once(
                                "
                .or_else(|visitor| ",
                            )
                            .chain(processed)
                            .chain(iter::once(")"))
                        })
                    })
                    .collect::<Result<_, _>>()
                    .map(Vec::into_iter)
                    .map(move |rest_of_entries| {
                        iter::once(PAIRS_GROUP_ENTRIES_PREPEND)
                            .chain(first_entry)
                            .chain(rest_of_entries.flatten())
                    })
            })
            .map(Either::Left)
            .map(|sources| PairsGroup {
                matcher_parameter_name: "matcher",
                sources,
            })
    } else {
        Ok(PairsGroup {
            matcher_parameter_name: "_",
            sources: Either::Right(iter::once("currency::visit_noone(visitor)")),
        })
    }
}

struct PairsGroup<I> {
    matcher_parameter_name: &'static str,
    sources: I,
}

fn in_pool_with<'r, 'dex_currencies, 'parent, 'name, Parents>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    parents: Parents,
    name: &'name str,
) -> Result<impl Iterator<Item = &'r str> + use<'r, Parents>>
where
    'dex_currencies: 'r,
    'name: 'r,
    Parents: Iterator<Item = &'parent str>,
{
    parents
        .map(|ticker| {
            ModuleAndName::resolve(
                current_module,
                protocol,
                host_currency,
                dex_currencies,
                ticker,
            )
            .map(|resolved| {
                [
                    "
    impl currency::InPoolWith<",
                    resolved.module(),
                    "::",
                    resolved.name(),
                    "> for ",
                    name,
                    " {}
",
                ]
            })
        })
        .collect::<Result<_, _>>()
        .map(Vec::into_iter)
        .map(Iterator::flatten)
}
