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

const NON_EXISTENT_DEX_CURRENCY: &'static str =
    "Queried ticker does not belong to any defined DEX currency!";

pub(super) fn write<'r, Report, Currencies>(
    build_report: Report,
    output_file: &'r Path,
    current_module: CurrentModule,
    protocol: &'r Protocol,
    host_currency: &'r CurrencyDefinition,
    dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies: Currencies,
    currencies_tree: &'r CurrenciesTree<'_, '_, '_, '_>,
) -> Result<()>
where
    Report: Write,
    Currencies: IntoIterator<Item = &'r str, IntoIter: 'r>,
{
    let GeneratedSources {
        matcher_parameter_name,
        maybe_visit,
        currency_definitions,
    } = generate_sources(
        build_report,
        output_file,
        protocol,
        host_currency,
        dex_currencies,
        currencies_tree,
        current_module,
        currencies.into_iter(),
    )?;

    let mut output_file = File::create(output_file).map(BufWriter::new)?;

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
    .context("Failed to write host's native currency implementation!")
}

fn generate_sources<'r, Report, Currencies>(
    mut build_report: Report,
    output_file: &'r Path,
    protocol: &'r Protocol,
    host_currency: &'r CurrencyDefinition,
    dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies_tree: &'r CurrenciesTree,
    current_module: CurrentModule,
    mut currencies: Currencies,
) -> Result<
    GeneratedSources<'r, 'r, impl Iterator<Item = &'r str>, impl Iterator<Item = Cow<'r, str>>>,
>
where
    Report: Write,
    Currencies: Iterator<Item = &'r str> + 'r,
{
    const MAYBE_VISIT_BODY_PREPEND: &'static str = "use currency::maybe_visit_member as visit;

    ";

    const CURRENCY_DEFINITIONS_PREPEND: &'static str = "
pub(crate) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
        PairsVisitor,
    };
    use sdk::schemars::JsonSchema;

    use crate::payment;
";

    const CURRENCY_DEFINITIONS_APPEND: &'static str = "}
";

    if let Some(ticker) = currencies.next() {
        let process_ticker1 = |ticker| {
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
        };

        let generate_currency_definition = |ticker| {
            generate_currency_definition(
                current_module,
                protocol,
                host_currency,
                dex_currencies,
                currencies_tree,
                ticker,
            )
        };

        let sources = iter::once(
            process_ticker1(ticker)
                .map(IntoIterator::into_iter)
                .map(Either::Left)
                .and_then(|processed_1| {
                    generate_currency_definition(ticker)
                        .map(|currency_definition| (processed_1, currency_definition))
                }),
        )
        .chain(currencies.map({
            let process_ticker1 = |ticker| {
                process_ticker1(ticker)
                    .map(|processed| {
                        iter::once(
                            "
        .or_else(|visitor| ",
                        )
                        .chain(processed)
                        .chain(iter::once(")"))
                    })
                    .map(Either::Right)
            };

            move |ticker| {
                process_ticker1(ticker).and_then(|processed_1| {
                    generate_currency_definition(ticker)
                        .map(|processed_ticker_2| (processed_1, processed_ticker_2))
                })
            }
        }))
        .collect::<Result<Vec<_>, _>>()?;

        let currencies_count = sources.len();

        let (maybe_visit_body, currency_definitions) = {
            let (maybe_visit_body, currency_definitions) =
                sources.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

            (
                iter::once(MAYBE_VISIT_BODY_PREPEND).chain(maybe_visit_body.into_iter().flatten()),
                iter::once(const { Cow::Borrowed(CURRENCY_DEFINITIONS_PREPEND) })
                    .chain(currency_definitions.into_iter().flatten())
                    .chain(iter::once(
                        const { Cow::Borrowed(CURRENCY_DEFINITIONS_APPEND) },
                    )),
            )
        };

        build_report
            .write_fmt(format_args!(
                "{output_file:?}: {currencies_count} currencies emitted.\n",
            ))
            .context("Failed to write build report!")
            .map(move |()| GeneratedSources {
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

struct GeneratedSources<'maybe_visit, 'currency_definitions, MaybeVisit, CurrencyDefinitions>
where
    MaybeVisit: Iterator<Item = &'maybe_visit str>,
    CurrencyDefinitions: Iterator<Item = Cow<'currency_definitions, str>>,
{
    matcher_parameter_name: &'static str,
    maybe_visit: MaybeVisit,
    currency_definitions: CurrencyDefinitions,
}

fn generate_currency_definition<'r>(
    current_module: CurrentModule,
    protocol: &'r Protocol,
    host_currency: &'r CurrencyDefinition,
    dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies_tree: &'r CurrenciesTree,
    ticker: &'r str,
) -> Result<impl Iterator<Item = Cow<'r, str>> + 'r> {
    let parents = currencies_tree.parents(ticker);

    let children = currencies_tree.children(ticker);

    [children, parents].into_iter().try_for_each({
        |paired_with| {
            if paired_with.contains(ticker) {
                Err(anyhow!("Currency cannot be in a pool with itself!"))
            } else {
                Ok(())
            }
        }
    })?;

    let &(ref name, currency) = dex_currencies
        .get(ticker)
        .context(NON_EXISTENT_DEX_CURRENCY)?;

    let pairs_group = {
        pairs_group(
            current_module,
            protocol,
            host_currency,
            dex_currencies,
            children.iter().copied(),
        )?
    };

    let in_pool_with = in_pool_with(
        current_module,
        protocol,
        host_currency,
        dex_currencies,
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

fn pairs_group<'r, 'currencies_map, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'currencies_map BTreeMap<&str, (String, &CurrencyDefinition)>,
    mut children: Children,
) -> Result<PairsGroup<'currencies_map, impl Iterator<Item = &'currencies_map str>>>
where
    Children: Iterator<Item = &'r str>,
{
    const PAIRS_GROUP_ENTRIES_PREPEND: &'static str = "use currency::maybe_visit_buddy as visit;

            ";

    fn pairs_group_entry<'r>(
        current_module: CurrentModule,
        protocol: &Protocol,
        host_currency: &CurrencyDefinition,
        dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
        ticker: &str,
    ) -> Result<impl IntoIterator<Item = &'r str>> {
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

struct PairsGroup<'sources, I>
where
    I: Iterator<Item = &'sources str>,
{
    matcher_parameter_name: &'static str,
    sources: I,
}

fn in_pool_with<'r, 'parent, Parents>(
    current_module: CurrentModule,
    protocol: &'r Protocol,
    host_currency: &'r CurrencyDefinition,
    dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
    parents: Parents,
    name: &'r str,
) -> Result<impl Iterator<Item = &'r str> + 'r>
where
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
