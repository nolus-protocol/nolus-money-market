use std::{borrow::Cow, iter};

use anyhow::{Context as _, Result, anyhow};

use crate::currencies_tree::{self, CurrenciesTree};

use super::super::super::generator;

pub(super) struct CurrencyDefinition<
    'currencies_tree,
    'parents_of,
    'parent,
    'children_of,
    'child,
    'generator,
    Generator,
> {
    currencies_tree: &'currencies_tree CurrenciesTree<'parents_of, 'parent, 'children_of, 'child>,
    generator: &'generator Generator,
}

impl<'currencies_tree, 'parents_of, 'parent, 'children_of, 'child, 'generator, Generator>
    CurrencyDefinition<
        'currencies_tree,
        'parents_of,
        'parent,
        'children_of,
        'child,
        'generator,
        Generator,
    >
{
    #[inline]
    pub const fn new(
        currencies_tree: &'currencies_tree CurrenciesTree<
            'parents_of,
            'parent,
            'children_of,
            'child,
        >,
        generator: &'generator Generator,
    ) -> Self {
        Self {
            currencies_tree,
            generator,
        }
    }
}

pub(super) struct GeneratedEntry<
    Variants,
    FirstEntry,
    HeadNextEntry,
    MiddleNextEntry,
    TailNextEntry,
    FilterMapEntry,
    FindMapEntry,
    CurrencyDefinition,
> {
    pub variant: Variants,
    pub first_entry: FirstEntry,
    pub head_next_entry: HeadNextEntry,
    pub middle_next_entry: MiddleNextEntry,
    pub tail_next_entry: TailNextEntry,
    pub filter_map_entry: FilterMapEntry,
    pub find_map_entry: FindMapEntry,
    pub currency_definition: CurrencyDefinition,
}

impl<
    'dex_currencies,
    'definition,
    'dex_currency_ticker,
    'dex_currency_definition,
    'currencies_tree,
    'parents,
    'parent,
    'generator,
    Generator,
> CurrencyDefinition<'currencies_tree, 'parents, 'parent, '_, '_, 'generator, Generator>
where
    'dex_currencies: 'definition,
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
    Generator: generator::Resolver<'dex_currencies, 'definition>
        + generator::GroupMembers<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
        + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
{
    pub(super) fn generate_entry<'r>(
        &self,
        ticker: &'r str,
    ) -> GeneratedEntryResult<
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = Cow<'r, str>>
        + use<
            'r,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
            'currencies_tree,
            'parent,
            'generator,
            Generator,
        >,
    >
    where
        'definition: 'r,
        'parent: 'r,
    {
        let parents = self.currencies_tree.parents(ticker);

        let children = self.currencies_tree.children(ticker);

        if [children.as_ref(), parents.as_ref()]
            .into_iter()
            .any(|paired_with| paired_with.contains(ticker))
        {
            Err(anyhow!("Currency cannot be in a pool with itself!"))
        } else {
            self.generate_entry_unchecked(ticker, children, parents)
        }
    }

    fn generate_entry_unchecked<'r, 'children, 'child>(
        &self,
        ticker: &'r str,
        children: &'children currencies_tree::Children<'child>,
        parents: &'parents currencies_tree::Parents<'parent>,
    ) -> GeneratedEntryResult<
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = &'r str> + use<'r, Generator>,
        impl Iterator<Item = Cow<'r, str>>
        + use<
            'r,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
            'currencies_tree,
            'children,
            'parents,
            'parent,
            'generator,
            Generator,
        >,
    >
    where
        'definition: 'r,
        'parent: 'r,
    {
        let resolved = self
            .generator
            .resolve(ticker)
            .context("Failed to generate currency definition sources!")?;

        let _ = self.generator.group_members(resolved.name())?;

        let pairs_group = self.generator.pairs_group(resolved.name(), parents)?;

        self.generator
            .in_pool_with(resolved.name(), children)
            .map(|in_pool_with| GeneratedEntry {
                variant: [
                    "
    ",
                    resolved.name(),
                    ",",
                ]
                .into_iter(),
                first_entry: ["Some(Self::", resolved.name(), ")"].into_iter(),
                head_next_entry: [
                    "
            Self::",
                    resolved.name(),
                    " => ",
                ]
                .into_iter(),
                middle_next_entry: [
                    "Some(Self::",
                    resolved.name(),
                    "),
            Self::",
                    resolved.name(),
                    " => ",
                ]
                .into_iter(),
                tail_next_entry: ["None,
        "]
                .into_iter(),
                filter_map_entry: [
                    "
            Self::",
                    resolved.name(),
                    " => filter_map.on::<self::definitions::",
                    resolved.name(),
                    ">(<self::definitions::",
                    resolved.name(),
                    " as currency::CurrencyDef>::dto()),",
                ]
                .into_iter(),
                find_map_entry: [
                    "
            Self::",
                    resolved.name(),
                    " => find_map.on::<self::definitions::",
                    resolved.name(),
                    ">(<self::definitions::",
                    resolved.name(),
                    " as currency::CurrencyDef>::dto()),",
                ]
                .into_iter(),
                currency_definition: currency_definition(
                    resolved.name(),
                    ticker,
                    resolved.definition(),
                )
                .chain(pairs_group.chain(in_pool_with).map(Cow::Borrowed)),
            })
    }
}

pub(super) type GeneratedEntryResult<
    Variants,
    FirstEntry,
    HeadNextEntry,
    MiddleNextEntry,
    TailNextEntry,
    FilterMapEntry,
    FindMapEntry,
    CurrencyDefinition,
> = Result<
    GeneratedEntry<
        Variants,
        FirstEntry,
        HeadNextEntry,
        MiddleNextEntry,
        TailNextEntry,
        FilterMapEntry,
        FindMapEntry,
        CurrencyDefinition,
    >,
>;

fn currency_definition<'r>(
    name: &'r str,
    ticker: &'r str,
    currency: &'r topology::CurrencyDefinition,
) -> impl Iterator<Item = Cow<'r, str>> {
    [
        r#"
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize,
        serde::Deserialize,
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct "#,
        name,
        r#"(currency::CurrencyDTO<super::super::Group>);

    impl currency::CurrencyDef for "#,
        name,
        r#" {
        type Group = super::super::Group;

        fn dto() -> &'static currency::CurrencyDTO<Self::Group> {
            const {
                &currency::CurrencyDTO::new(
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
                )
            }
        }
    }
"#,
            )
        },
    ))
}
