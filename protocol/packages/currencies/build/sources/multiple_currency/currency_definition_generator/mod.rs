use std::{borrow::Cow, iter};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{
    currencies_tree::CurrenciesTree, protocol::Protocol, sources::module_and_name::CurrentModule,
    subtype_lifetime::SubtypeLifetime,
};

use super::{DexCurrencies, NON_EXISTENT_DEX_CURRENCY};

mod in_pool_with;
mod pairs_group;

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
        visitor_parameter_name: &'static str,
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
            self.generate_entry_unchecked(
                ticker,
                visitor_parameter_name,
                children.iter().copied(),
                parents.iter().copied(),
            )
        }
    }

    fn generate_entry_unchecked<'r, 'child, 'parent, Children, Parents>(
        &self,
        ticker: &'r str,
        visitor_parameter_name: &'static str,
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
                    visitor_parameter_name,
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
                        Self::finalize_entry(name, ticker, currency, pairs_group, in_pool_with)
                    })
                })
            })
    }

    fn finalize_entry<'r, 'pairs_group, 'in_pool_with, PairsGroup, InPoolWith>(
        name: &'r str,
        ticker: &'r str,
        currency: &'r CurrencyDefinition,
        pairs_group: pairs_group::PairsGroup<PairsGroup>,
        in_pool_with: InPoolWith,
    ) -> impl Iterator<Item = Cow<'r, str>> + use<'r, 'pairs_group, 'in_pool_with, PairsGroup, InPoolWith>
    where
        'pairs_group: 'r,
        'in_pool_with: 'r,
        PairsGroup: Iterator<Item = &'pairs_group str>,
        InPoolWith: Iterator<Item = &'in_pool_with str>,
    {
        [
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
                r#": &M, "#,
                pairs_group.visitor_parameter_name,
                r#": V) -> MaybePairsVisitorResult<V>
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
                .map(SubtypeLifetime::subtype)
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
                .map(SubtypeLifetime::subtype)
                .map(Cow::Borrowed),
        )
    }
}
