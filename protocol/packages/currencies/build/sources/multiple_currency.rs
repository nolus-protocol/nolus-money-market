use std::{borrow::Cow, collections::BTreeMap, fs, io::Write, iter, path::Path};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{
    currencies_tree::CurrenciesTree,
    either::Either,
    iter::{TransposeResult, TryCollect},
    protocol::Protocol,
};

use super::module_and_name::{CurrentModule, ModuleAndName};

const SOURCE_1_BASE: &'static str = "use currency::maybe_visit_member as visit;

    ";

const SOURCE_2_BASE: &'static str = "
pub(crate) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
        PairsVisitor,
    };
    use sdk::schemars::JsonSchema;

    use crate::payment;
";

pub(super) fn write<'currency, Report, Currencies>(
    build_report: Report,
    output_file: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies: Currencies,
    currencies_tree: &CurrenciesTree<'_, '_, '_, '_>,
    current_module: CurrentModule,
) -> Result<()>
where
    Report: Write,
    Currencies: IntoIterator<Item = &'currency str>,
{
    let F {
        maybe_visit_body,
        currencies,
    } = ffffffffffffffffffffffff(
        build_report,
        output_file,
        protocol,
        host_currency,
        dex_currencies,
        currencies_tree,
        current_module,
        currencies.into_iter(),
    )?;

    fs::write(
        output_file,
        format!(
            r#"// @generated

use currency::{{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf}};

use crate::payment;

pub(super) fn maybe_visit<M, V, VisitedG>(
    {matcher}: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    super::Group: MemberOf<VisitedG>,
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = payment::Group>,
{{
    {maybe_visit_body}
}}
{currencies}"#,
            matcher = if matches!(maybe_visit_body, Cow::Borrowed { .. }) {
                "_"
            } else {
                "matcher"
            }
        ),
    )
    .context("Failed to write host's native currency implementation!")
}

fn ffffffffffffffffffffffff<'currency, Report, Currencies>(
    mut build_report: Report,
    output_file: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies_tree: &CurrenciesTree,
    current_module: CurrentModule,
    mut currencies: Currencies,
) -> Result<F>
where
    Report: Write,
    Currencies: Iterator<Item = &'currency str>,
{
    const NON_EXISTENT_DEX_CURRENCY: &'static str =
        "Queried ticker does not belong to any defined DEX currency!";

    Ok(if let Some(ticker) = currencies.next() {
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

        let mut maybe_visit_body: String = SOURCE_1_BASE.into();

        let mut currencies_string: String = SOURCE_2_BASE.into();

        let process_ticker2 = |ticker| {
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
                    protocol,
                    host_currency,
                    dex_currencies,
                    children.iter().copied(),
                    current_module,
                )?
            };

            let in_pool_with = {
                let mut in_pool_with = String::new();

                for &ticker in parents {
                    let resolved = ModuleAndName::resolve(
                        protocol,
                        host_currency,
                        dex_currencies,
                        ticker,
                        current_module,
                    )?;

                    in_pool_with.extend([
                        "
    impl currency::InPoolWith<",
                        resolved.module(),
                        "::",
                        resolved.name(),
                        "> for ",
                        name,
                        " {}
",
                    ]);
                }

                in_pool_with
            };

            Ok(format!(
                r#"
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    #[schemars(crate = "sdk::schemars")]
    pub struct {name}(CurrencyDTO<super::super::Group>);

    impl CurrencyDef for {name} {{
        type Group = super::super::Group;

        fn definition() -> &'static Self {{
            const {{
                &Self(CurrencyDTO::new(
                    const {{
                        &Definition::new(
                            {ticker:?},
                            // {host_path}
                            {host_symbol:?},
                            // {dex_path}
                            {dex_symbol:?},
                            {decimals},
                        )
                    }},
                ))
            }}
        }}

        fn dto(&self) -> &CurrencyDTO<Self::Group> {{
            &self.0
        }}
    }}

    impl PairsGroup for {name} {{
        type CommonGroup = payment::Group;

        fn maybe_visit<M, V>({matcher}: &M, visitor: V) -> MaybePairsVisitorResult<V>
        where
            M: Matcher,
            V: PairsVisitor<Pivot = Self>,
        {{
            {pairs_group}
        }}
    }}
{in_pool_with}"#,
                host_path = currency.host().path(),
                host_symbol = currency.host().symbol(),
                dex_path = currency.dex().path(),
                dex_symbol = currency.dex().symbol(),
                decimals = currency.decimal_digits(),
                matcher = if matches!(pairs_group, Cow::Borrowed(_)) {
                    "_"
                } else {
                    "matcher"
                },
            ))
        };

        iter::once(
            process_ticker1(ticker)
                .map(IntoIterator::into_iter)
                .map(Either::Left)
                .and_then(|processed_1| {
                    process_ticker2(ticker).map(|processed_2| (processed_1, processed_2))
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
                    process_ticker2(ticker)
                        .map(|processed_ticker_2| (processed_1, processed_ticker_2))
                })
            }
        }))
        .try_fold(0_usize, |currencies_count, result| {
            result.map(|(processed_1, processed_2)| {
                maybe_visit_body.extend(processed_1);

                currencies_string.push_str(&processed_2);

                currencies_count + 1
            })
        })
        .inspect(|_| {
            currencies_string.push_str(
                "}
",
            );
        })
        .and_then(|currencies_count| {
            build_report
                .write_fmt(format_args!(
                    "{output_file:?}: {currencies_count} currencies emitted.\n",
                ))
                .context("Failed to write build report!")
        })
        .map(|()| F {
            maybe_visit_body: Cow::Owned(maybe_visit_body),
            currencies: currencies_string,
        })?
    } else {
        const {
            F {
                maybe_visit_body: Cow::Borrowed("currency::visit_noone(visitor)"),
                currencies: String::new(),
            }
        }
    })
}

struct F {
    maybe_visit_body: Cow<'static, str>,
    currencies: String,
}

fn pairs_group<'r, Children>(
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    children: Children,
    current_module: CurrentModule,
) -> Result<Cow<'static, str>>
where
    Children: IntoIterator<Item = &'r str>,
{
    let mut children = children.into_iter();

    if let Some(ticker) = children.next() {
        let process_ticker = |ticker: &str| {
            ModuleAndName::resolve(
                protocol,
                host_currency,
                dex_currencies,
                ticker,
                current_module,
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
        };

        iter::once(
            const {
                Ok("use currency::maybe_visit_buddy as visit;

            ")
            },
        )
        .chain(process_ticker(ticker).transpose())
        .chain(children.flat_map(|ticker| {
            process_ticker(ticker)
                .map(|processed| {
                    iter::once(
                        "
                .or_else(|visitor| ",
                    )
                    .chain(processed)
                    .chain(iter::once(")"))
                })
                .transpose()
        }))
        .try_collect()
        .map(Cow::Owned)
    } else {
        const { Ok(Cow::Borrowed("currency::visit_noone(visitor)")) }
    }
}
