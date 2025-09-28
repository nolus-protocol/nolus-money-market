use std::iter;

use anyhow::Result;
use either::Either;

use topology::HostCurrency;

use crate::{
    currencies_tree::Parents,
    protocol::Protocol,
    sources::resolved_currency::{CurrentModule, ResolvedCurrency},
    subtype_lifetime::SubtypeLifetime,
};

use super::{Captures, DexCurrencies, Generator};

pub(in super::super) trait PairsGroup<
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: &'parents Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r;
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        '_,
        '_,
        '_,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
    >
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        _: &'name str,
        _: &'parents Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r,
    {
        const { Ok(iter::empty()) }
    }
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<'_, '_, '_, 'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, true>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    fn pairs_group<'r, 'name, 'parents, 'parent>(
        &self,
        name: &'name str,
        parents: &'parents Parents<'parent>,
    ) -> Result<
        impl Iterator<Item = &'r str>
        + Captures<&'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>>
        + Captures<&'name str>
        + Captures<&'parents Parents<'parent>>,
    >
    where
        'dex_currencies: 'r,
        'name: 'r,
        'parent: 'r,
    {
        pairs_group(
            self.current_module,
            self.static_context.protocol,
            self.static_context.host_currency,
            self.static_context.dex_currencies,
            name,
            parents.iter().copied(),
        )
    }
}

fn pairs_group<'r, 'dex_currencies, 'parent, Parents>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &HostCurrency,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    name: &'r str,
    mut parents: Parents,
) -> Result<impl Iterator<Item = &'r str> + use<'r, 'parent, Parents>>
where
    'dex_currencies: 'r,
    'parent: 'r,
    Parents: Iterator<Item = &'parent str> + Clone,
{
    const ENUM_IDENT: &str = "Pairs";

    if let Some(ticker) = parents.next() {
        fn find_map_source_segment<'r, 'ticker, 'dex_currencies>(
            ticker: &'ticker str,
            current_module: CurrentModule,
            protocol: &Protocol,
            host_currency: &HostCurrency,
            dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
        ) -> Result<impl IntoIterator<Item = &'r str> + use<'r>>
        where
            'ticker: 'r,
            'dex_currencies: 'r,
        {
            ResolvedCurrency::resolve(
                current_module,
                protocol,
                host_currency,
                dex_currencies,
                ticker,
            )
            .map(|resolved| {
                [
                    "
                        Self::",
                    ticker,
                    " => find_map.on::<",
                    resolved.module(),
                    "::",
                    resolved.name(),
                    ">(<",
                    resolved.module(),
                    "::",
                    resolved.name(),
                    " as currency::CurrencyDef>::dto()),",
                ]
            })
        }

        let source = [
            r#"#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
            enum "#,
            ENUM_IDENT,
            r#" {
                "#,
            ticker,
            ",",
        ]
        .into_iter()
        .chain(parents.clone().flat_map(|ticker| {
            [
                "
                ",
                ticker,
                ",",
            ]
        }))
        .chain([
            r#"
            }

            impl currency::PairsGroupMember for "#,
            ENUM_IDENT,
            r#" {
                type Group = "#,
            name,
            r#";

                fn first() -> Option<Self> {
                    Some(Self::"#,
            ticker,
            r#")
                }

                fn next(&self) -> Option<Self> {
                    match *self {
                        Self::"#,
            ticker,
        ])
        .chain(parents.clone().flat_map(|ticker| {
            [
                " => Some(Self::",
                ticker,
                "),
                        Self::",
                ticker,
            ]
        }))
        .chain(iter::once(
            " => None,
                    }
                }

                fn find_map<FindMap>(
                    &self,
                    find_map: FindMap,
                ) -> Result<<FindMap as currency::PairsFindMapT>::Outcome, FindMap>
                where
                    FindMap: currency::PairsFindMapT<Pivot = Self::Group>,
                {
                    match *self {",
        ));

        iter::once(ticker)
            .chain(parents)
            .try_fold(vec![], |mut find_map_source, ticker| {
                find_map_source_segment(
                    ticker,
                    current_module,
                    protocol,
                    host_currency,
                    dex_currencies,
                )
                .map(|source| {
                    find_map_source.extend(source);

                    find_map_source
                })
            })
            .map(|find_map_source| {
                source.chain(find_map_source).chain(iter::once(
                    "
                    }
                }
            }",
                ))
            })
            .map(Either::Left)
    } else {
        Ok(Either::Right(
            [
                r#"enum "#,
                ENUM_IDENT,
                r#" {}

            impl currency::PairsGroupMember for "#,
                ENUM_IDENT,
                r#" {
                type Group = "#,
                name,
                r#";

                fn first() -> Option<Self> {
                    None
                }

                fn next(&self) -> Option<Self> {
                    match *self {}
                }

                fn find_map<FindMap>(
                    &self,
                    _: FindMap,
                ) -> Result<<FindMap as currency::PairsFindMapT>::Outcome, FindMap>
                where
                    FindMap: currency::PairsFindMapT<Pivot = Self::Group>,
                {
                    match *self {}
                }
            }"#,
            ]
            .into_iter(),
        ))
    }
    .map(|sources| {
        [
            r#"
    impl currency::PairsGroup for "#,
            name,
            r#" {
        type CommonGroup = crate::payment::Group;

        fn find_map<FindMap>(
            find_map: FindMap,
        ) -> Result<<FindMap as currency::PairsFindMapT>::Outcome, FindMap>
        where
            FindMap: currency::PairsFindMapT<Pivot = Self>,
        {
            "#,
        ]
        .into_iter()
        .chain(sources.map(SubtypeLifetime::subtype))
        .chain([
            r#"

            currency::pairs_find_map::<"#,
            ENUM_IDENT,
            r#", _>(find_map)
        }
    }
"#,
        ])
    })
}
