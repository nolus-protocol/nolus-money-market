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

use crate::{currencies_tree::CurrenciesTree, either::Either, protocol::Protocol};

use super::module_and_name::{CurrentModule, ModuleAndName};

const NON_EXISTENT_DEX_CURRENCY: &str =
    "Queried ticker does not belong to any defined DEX currency!";

pub(super) struct SourcesGenerator<
    'output_file,
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
    Report,
    Currencies,
> {
    pub build_report: Report,
    pub output_file: &'output_file Path,
    pub current_module: CurrentModule,
    pub protocol: &'protocol Protocol,
    pub host_currency: &'host_currency CurrencyDefinition,
    pub dex_currencies: &'dex_currencies BTreeMap<
        &'dex_currency_ticker str,
        (String, &'dex_currency_definition CurrencyDefinition),
    >,
    pub currencies: Currencies,
    pub currencies_tree:
        &'currencies_tree CurrenciesTree<'parents_map, 'parent, 'children_map, 'child>,
}

impl<
        'output_file,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        'currency,
        'currencies_tree,
        'parents_map,
        'parent,
        'children_map,
        'child,
        Report,
        Currencies,
    >
    SourcesGenerator<
        'output_file,
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
        Report,
        Currencies,
    >
where
    Report: Write,
    Currencies: IntoIterator<Item = &'currency str>,
{
    pub fn generate_and_commit(self) -> Result<()> {
        let output_file_path = self.output_file;

        let GeneratedSources {
            matcher_parameter_name,
            maybe_visit,
            currency_definitions,
        } = self.generate_sources()?;

        let mut output_file = File::create(output_file_path).map(BufWriter::new)?;

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
        .chain(maybe_visit)
        .chain(iter::once(
            "
}
",
        ))
        .map(Cow::Borrowed)
        .chain(currency_definitions)
        .try_for_each(|segment| output_file.write_all(segment.as_bytes()))
        .and_then(|()| output_file.flush())
        .with_context(|| {
            format!("Failed to write generated sources for output file {output_file_path:?}!")
        })
    }

    fn generate_sources<'r>(
        mut self,
    ) -> Result<
        GeneratedSources<
            impl Iterator<Item = &'r str> + use<'r, Report, Currencies>,
            impl Iterator<Item = Cow<'r, str>> + use<'r, Report, Currencies>,
        >,
    >
    where
        'dex_currencies: 'r,
        'currency: 'r,
    {
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

        let mut currencies = self.currencies.into_iter();

        if let Some(ticker) = currencies.next() {
            let currency_definition_generator = CurrencyDefinitionGenerator {
                current_module: self.current_module,
                protocol: self.protocol,
                host_currency: self.host_currency,
                dex_currencies: self.dex_currencies,
                currencies_tree: self.currencies_tree,
            };

            let sources = iter::once(
                maybe_visit_entry(self.dex_currencies, ticker)
                    .map(IntoIterator::into_iter)
                    .map(Either::Left)
                    .and_then(|maybe_visit_entry| {
                        currency_definition_generator
                            .generate(ticker)
                            .map(|currency_definition| (maybe_visit_entry, currency_definition))
                    }),
            )
            .chain(currencies.map({
                let maybe_visit_entry = |ticker| {
                    maybe_visit_entry(self.dex_currencies, ticker)
                        .map(|maybe_visit_entry| {
                            iter::once(
                                "
        .or_else(|visitor| ",
                            )
                            .chain(maybe_visit_entry)
                            .chain(iter::once(")"))
                        })
                        .map(Either::Right)
                };

                move |ticker| {
                    maybe_visit_entry(ticker).and_then(|maybe_visit_entry| {
                        currency_definition_generator
                            .generate(ticker)
                            .map(|currency_definition| (maybe_visit_entry, currency_definition))
                    })
                }
            }))
            .collect::<Result<Vec<_>, _>>()?;

            let currencies_count = sources.len();

            let (maybe_visit_body, currency_definitions) = {
                let (maybe_visit_body, currency_definitions): (Vec<_>, Vec<_>) =
                    sources.into_iter().unzip();

                (
                    iter::once(MAYBE_VISIT_BODY_PREPEND)
                        .chain(maybe_visit_body.into_iter().flatten()),
                    iter::once(CURRENCY_DEFINITIONS_PREPEND)
                        .chain(currency_definitions.into_iter().flatten())
                        .chain(iter::once(CURRENCY_DEFINITIONS_APPEND)),
                )
            };

            self.build_report
                .write_fmt(format_args!(
                    "{output_file:?}: {currencies_count} currencies emitted.\n",
                    output_file = self.output_file,
                ))
                .context("Failed to write build report!")
                .map(|()| GeneratedSources {
                    matcher_parameter_name: "matcher",
                    maybe_visit: Either::Left(maybe_visit_body),
                    currency_definitions: Either::Left(currency_definitions),
                })
        } else {
            Ok(GeneratedSources {
                matcher_parameter_name: "_",
                maybe_visit: Either::Right(iter::once("currency::visit_noone(visitor)")),
                currency_definitions: Either::Right(iter::once(const { Cow::Borrowed("") })),
            })
        }
    }
}

fn maybe_visit_entry<'dex_currencies>(
    dex_currencies: &'dex_currencies BTreeMap<&str, (String, &CurrencyDefinition)>,
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
    matcher_parameter_name: &'static str,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
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
    dex_currencies: &'dex_currencies BTreeMap<
        &'dex_currency_ticker str,
        (String, &'dex_currency_definition CurrencyDefinition),
    >,
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
    CurrencyDefinitionGenerator<
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
    fn generate<'r, 'ticker>(
        &self,
        ticker: &'ticker str,
    ) -> Result<impl Iterator<Item = Cow<'r, str>> + use<'r>>
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
            parents.iter().copied().collect::<Vec<_>>().into_iter(),
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
        .chain(
            pairs_group
                .sources
                .collect::<Vec<_>>()
                .into_iter()
                .map(Cow::Borrowed),
        )
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
        .chain(
            in_pool_with
                .collect::<Vec<_>>()
                .into_iter()
                .map(Cow::Borrowed),
        ))
    }
}

fn pairs_group<'dex_currencies, 'child, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies BTreeMap<&str, (String, &CurrencyDefinition)>,
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
        dex_currencies: &'dex_currencies BTreeMap<&str, (String, &CurrencyDefinition)>,
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
    dex_currencies: &'dex_currencies BTreeMap<&str, (String, &CurrencyDefinition)>,
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
                .into_iter()
            })
        })
        .collect::<Result<_, _>>()
        .map(Vec::into_iter)
        .map(Iterator::flatten)
}
