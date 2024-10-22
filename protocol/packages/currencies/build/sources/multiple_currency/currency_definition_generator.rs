use std::borrow::Cow;

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, protocol::Protocol};

use super::{
    super::{
        currency_definition, in_pool_with, module_and_name::CurrentModule, pairs_group,
        DexCurrencies,
    },
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
            self.generate_entry_unchecked(ticker, children.iter().copied(), parents.iter().copied())
        }
    }

    fn generate_entry_unchecked<'r, 'child, 'parent, Children, Parents>(
        &self,
        ticker: &'r str,
        children: Children,
        parents: Parents,
    ) -> Result<impl Iterator<Item = Cow<'r, str>> + use<'r, 'currencies_tree, Children, Parents>>
    where
        'dex_currencies: 'r,
        Children: Iterator<Item = &'child str>,
        Parents: Iterator<Item = &'parent str>,
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
                    children,
                )
                .and_then(|pairs_group| {
                    in_pool_with::in_pool_with(
                        self.current_module,
                        self.protocol,
                        self.host_currency,
                        self.dex_currencies,
                        name,
                        parents,
                    )
                    .map(|in_pool_with| {
                        currency_definition::currency_definition(name, ticker, currency).chain(
                            pairs_group
                                .chain(in_pool_with)
                                .map(|value| Cow::Borrowed(value)),
                        )
                    })
                })
            })
    }
}
