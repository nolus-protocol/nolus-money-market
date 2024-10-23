use std::{borrow::Cow, collections::BTreeSet, iter};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, protocol::Protocol};

use super::{
    super::super::{in_pool_with, module_and_name::CurrentModule, pairs_group, DexCurrencies},
    NON_EXISTENT_DEX_CURRENCY,
};

pub(super) struct CurrencyDefinitionGenerator<
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
    pub(super) const fn new(
        current_module: CurrentModule,
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
            current_module,
            protocol,
            host_currency,
            dex_currencies,
            currencies_tree,
        }
    }
}

impl<'dex_currencies, 'currencies_tree>
    CurrencyDefinitionGenerator<'_, '_, 'dex_currencies, '_, '_, 'currencies_tree, '_, '_, '_, '_>
{
    pub(super) fn generate_entry<'r>(
        &self,
        ticker: &'r str,
    ) -> Result<impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree>>
    where
        'dex_currencies: 'r,
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

    fn generate_entry_unchecked<'r, 'children_map, 'parents_map>(
        &self,
        ticker: &'r str,
        children: &'children_map BTreeSet<&str>,
        parents: &'parents_map BTreeSet<&str>,
    ) -> Result<
        impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree, 'children_map, 'parents_map>,
    >
    where
        'dex_currencies: 'r,
    {
        self.dex_currencies
            .get(ticker)
            .context(NON_EXISTENT_DEX_CURRENCY)
            .and_then(|&(ref name, currency)| {
                pairs_group::pairs_group(
                    self.current_module,
                    self.protocol,
                    self.host_currency,
                    self.dex_currencies,
                    name,
                    children.iter().copied(),
                )
                .and_then(|pairs_group| {
                    in_pool_with::in_pool_with(
                        self.current_module,
                        self.protocol,
                        self.host_currency,
                        self.dex_currencies,
                        name,
                        parents.iter().copied(),
                    )
                    .map(|in_pool_with| {
                        currency_definition(name, ticker, currency)
                            .chain(pairs_group.chain(in_pool_with).map(Cow::Borrowed))
                    })
                })
            })
    }
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
