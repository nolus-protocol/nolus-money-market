use std::{borrow::Cow, collections::BTreeSet, iter};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::currencies_tree::CurrenciesTree;

use super::super::super::{Generator, Resolver};

pub(super) struct CurrencyDefinitionGenerator<
    'currencies_tree,
    'parents_map,
    'parent,
    'children_map,
    'child,
    'generator,
    Generator,
> {
    pub currencies_tree:
        &'currencies_tree CurrenciesTree<'parents_map, 'parent, 'children_map, 'child>,
    pub generator: &'generator Generator,
    pub visit_function: &'static str,
    pub matcher_parameter: &'static str,
    pub visitor_parameter: &'static str,
}

impl<
        'host_currency,
        'dex_currencies,
        'definition,
        'dex_currency_ticker,
        'dex_currency_definition,
        'currencies_tree,
        'generator,
        Generator,
    > CurrencyDefinitionGenerator<'currencies_tree, '_, '_, '_, '_, 'generator, Generator>
where
    'host_currency: 'definition,
    'dex_currencies: 'definition,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    Generator: Resolver<'dex_currencies, 'definition>
        + self::Generator<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
{
    pub(super) fn generate_entry<'r>(
        &self,
        ticker: &'r str,
    ) -> Result<
        Entry<
            impl IntoIterator<Item = &'dex_currencies str> + use<'dex_currencies, Generator>,
            impl Iterator<Item = Cow<'r, str>>
                + use<
                    'r,
                    'dex_currencies,
                    'dex_currency_ticker,
                    'dex_currency_definition,
                    'currencies_tree,
                    'generator,
                    Generator,
                >,
        >,
    >
    where
        'definition: 'r,
    {
        let parents = self.currencies_tree.parents(ticker);

        let children = self.currencies_tree.children(ticker);

        if [children, parents]
            .into_iter()
            .any(|paired_with| paired_with.contains(ticker))
        {
            Err(anyhow!("Currency cannot be in a pool with itself!"))
        } else {
            self.generate_entry_unchecked(ticker, children, parents)
        }
    }

    fn generate_entry_unchecked<'r, 'children, 'child, 'parents, 'parent>(
        &self,
        ticker: &'r str,
        children: &'children BTreeSet<&'child str>,
        parents: &'parents BTreeSet<&'parent str>,
    ) -> Result<
        Entry<
            impl IntoIterator<Item = &'dex_currencies str> + use<'dex_currencies, Generator>,
            impl Iterator<Item = Cow<'r, str>>
                + use<
                    'r,
                    'dex_currencies,
                    'dex_currency_ticker,
                    'dex_currency_definition,
                    'currencies_tree,
                    'children,
                    'parents,
                    'generator,
                    Generator,
                >,
        >,
    >
    where
        'definition: 'r,
    {
        self.generator
            .resolve(ticker)
            .context("Failed to generate currency definition sources!")
            .and_then(|resolved| {
                self.generator
                    .pairs_group(resolved.name(), children)
                    .and_then(|pairs_group| {
                        self.generator
                            .in_pool_with(resolved.name(), parents)
                            .map(|in_pool_with| Entry {
                                maybe_visit: [
                                    self.visit_function,
                                    "::<_, self::",
                                    resolved.name(),
                                    ", VisitedG, _>(",
                                    self.matcher_parameter,
                                    ", ",
                                    self.visitor_parameter,
                                    ")",
                                ],
                                currency_definition: currency_definition(
                                    resolved.name(),
                                    ticker,
                                    resolved.definition(),
                                )
                                .chain(pairs_group.chain(in_pool_with).map(Cow::Borrowed)),
                            })
                    })
            })
    }
}

pub(super) struct Entry<MaybeVisit, CurrencyDefinition> {
    pub maybe_visit: MaybeVisit,
    pub currency_definition: CurrencyDefinition,
}

fn currency_definition<'r>(
    name: &'r str,
    ticker: &'r str,
    currency: &'r CurrencyDefinition,
) -> impl Iterator<Item = Cow<'r, str>> + use<'r> {
    [
        r#"
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize,
    serde::Deserialize, sdk::schemars::JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct "#,
        name,
        r#"(currency::CurrencyDTO<super::Group>);

impl currency::CurrencyDef for "#,
        name,
        r#" {
    type Group = super::Group;

    fn definition() -> &'static Self {
        const {
            &Self(currency::CurrencyDTO::new(
                const {
                    &currency::Definition::new(
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
    .chain(iter::once(
        const {
            Cow::Borrowed(
                r#",
                    )
                },
            ))
        }
    }

    fn dto(&self) -> &currency::CurrencyDTO<Self::Group> {
        &self.0
    }
}
"#,
            )
        },
    ))
}
