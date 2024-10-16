use std::{borrow::Cow, collections::BTreeMap, fs, io::Write, iter, path::Path};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, either::Either, protocol::Protocol};

use super::module_and_name::ModuleAndName;

pub(super) fn write<'currency, Report, Currencies>(
    mut build_report: Report,
    output_file: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies: Currencies,
    currencies_tree: &CurrenciesTree<'_, '_, '_, '_>,
) -> Result<()>
where
    Report: Write,
    Currencies: IntoIterator<Item = &'currency str> + Clone,
{
    struct F {
        maybe_visit_body: Cow<'static, str>,
        currencies: String,
    }

    let mut currencies = currencies.into_iter();

    let F {
        maybe_visit_body,
        currencies,
    } = if let Some(ticker) = currencies.next() {
        let process_ticker1 = |source: &mut String, ticker| {
            // TODO
            dex_currencies.get(ticker).context("TODO").map(|(name, _)| {
                source.extend([
                    "maybe_visit_member::<_, definitions::",
                    name,
                    ", VisitedG, _>(matcher, visitor)",
                ]);
            })
        };

        let mut source1: String = "use currency::maybe_visit_member;

"
        .into();

        let mut source2: String = "
pub(crate) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
        PairsVisitor,
    };
    use sdk::schemars::JsonSchema;

    use crate::payment;
"
        .into();

        let mut process_ticker2 = |ticker| -> Result<()> {
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

            // TODO
            let &(ref name, currency) = dex_currencies.get(ticker).context("TODO")?;

            let pairs_group = {
                let mut children = children.iter().copied();

                if let Some(ticker) = children.next() {
                    let process_ticker = |ticker: &str| {
                        ModuleAndName::resolve(protocol, host_currency, dex_currencies, ticker)
                            .map(|resolved| {
                                [
                                    "currency::maybe_visit_buddy::<crate::",
                                    resolved.module(),
                                    "::",
                                    resolved.name(),
                                    ", _, _>(matcher, visitor)",
                                ]
                            })
                            .transpose()
                    };

                    process_ticker(ticker)
                        .chain(children.flat_map(|ticker| {
                            iter::once(
                                const {
                                    Ok("
                .or_else(|visitor| ")
                                },
                            )
                            .chain(process_ticker(ticker))
                            .chain(iter::once(const { Ok(")") }))
                        }))
                        .try_collect()
                        .map(Cow::Owned)?
                } else {
                    const { Cow::Borrowed("currency::visit_noone(visitor)") }
                }
            };

            let in_pool_with = {
                let mut in_pool_with = String::new();

                for &ticker in parents {
                    let resolved =
                        ModuleAndName::resolve(protocol, host_currency, dex_currencies, ticker)?;

                    in_pool_with.extend([
                        "
    impl currency::InPoolWith<crate::",
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

            source2.push_str(&format!(
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
            ));

            Ok(())
        };

        process_ticker1(&mut source1, ticker)
            .and_then(|()| process_ticker2(ticker))
            .map(|()| {
                |ticker| {
                    source1.push_str(
                        "
        .or_else(|visitor| ",
                    );

                    process_ticker1(&mut source1, ticker).inspect(|_: &()| {
                        source1.push(')');
                    })
                }
            })
            .and_then(|mut process_ticker1| {
                currencies.try_fold(1, |currencies_count, ticker| {
                    process_ticker1(ticker)
                        .and_then(|()| process_ticker2(ticker))
                        .map(|()| currencies_count + 1)
                })
            })
            .and_then(|currencies_count| {
                build_report
                    .write_fmt(format_args!(
                        "{output_file:?}: {currencies_count} currencies emitted.\n",
                    ))
                    .context("Failed to write build report!")
            })
            .inspect(|_: &()| {
                source2.push_str(
                    "}
",
                );
            })
            .map(|()| F {
                maybe_visit_body: Cow::Owned(source1),
                currencies: source2,
            })?
    } else {
        const {
            F {
                maybe_visit_body: Cow::Borrowed("currency::visit_noone(visitor)"),
                currencies: String::new(),
            }
        }
    };

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

trait Transpose {
    type Ok;

    type Err;

    fn transpose(self) -> impl Iterator<Item = Result<Self::Ok, Self::Err>>;
}

impl<T, E> Transpose for Result<T, E>
where
    T: IntoIterator,
{
    type Ok = T::Item;

    type Err = E;

    fn transpose(self) -> impl Iterator<Item = Result<T::Item, E>> {
        match self {
            Ok(value) => Either::Left(value.into_iter().map(Ok)),
            Err(error) => Either::Right(iter::once(Err(error))),
        }
    }
}

trait TryExtend<T>: Sized {
    fn try_extend<I, E>(self, iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>;
}

impl<T> TryExtend<T> for String
where
    T: AsRef<str>,
{
    fn try_extend<I, E>(self, iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>,
    {
        iter.into_iter().try_fold(self, |mut accumulator, element| {
            element.map(|element| {
                accumulator.push_str(element.as_ref());

                accumulator
            })
        })
    }
}

pub trait TryFromIterator<T>: Sized {
    fn try_from_iter<I, E>(iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>;
}

impl<T> TryFromIterator<T> for String
where
    T: AsRef<str>,
{
    #[inline]
    fn try_from_iter<I, E>(iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>,
    {
        String::new().try_extend(iter)
    }
}

pub trait TryCollect: Iterator {
    fn try_collect<T, U, E>(self) -> Result<T, E>
    where
        Self: Iterator<Item = Result<U, E>>,
        T: TryFromIterator<U>;
}

impl<I> TryCollect for I
where
    Self: Iterator,
{
    #[inline]
    fn try_collect<T, U, E>(self) -> Result<T, E>
    where
        Self: Iterator<Item = Result<U, E>>,
        T: TryFromIterator<U>,
    {
        T::try_from_iter(self)
    }
}
