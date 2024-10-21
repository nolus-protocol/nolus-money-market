use std::iter;

use anyhow::Result;

use topology::CurrencyDefinition;

use crate::{
    either::Either,
    protocol::Protocol,
    sources::module_and_name::{CurrentModule, ModuleAndName},
};

use super::DexCurrencies;

pub(super) fn pairs_group<'dex_currencies, 'child, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    mut children: Children,
) -> Result<PairsGroup<impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, Children>>>
where
    Children: Iterator<Item = &'child str>,
{
    if let Some(ticker) = children.next() {
        non_empty_pairs_group(
            current_module,
            protocol,
            host_currency,
            dex_currencies,
            ticker,
            children,
        )
        .map(
            |PairsGroup {
                 matcher_parameter_name,
                 sources,
             }| PairsGroup {
                matcher_parameter_name,
                sources: Either::Left(sources),
            },
        )
    } else {
        Ok(PairsGroup {
            matcher_parameter_name: "_",
            sources: Either::Right(iter::once("currency::visit_noone(visitor)")),
        })
    }
}

fn non_empty_pairs_group<'dex_currencies, 'child, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    ticker: &str,
    children: Children,
) -> Result<PairsGroup<impl Iterator<Item = &'dex_currencies str> + use<'dex_currencies, Children>>>
where
    Children: Iterator<Item = &'child str>,
{
    const PAIRS_GROUP_ENTRIES_PREPEND: &str = "use currency::maybe_visit_buddy as visit;

            ";

    let process_ticker = |ticker: &str| {
        entry(
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
                    process_ticker(ticker).map(|entry| {
                        iter::once(
                            "
                .or_else(|visitor| ",
                        )
                        .chain(entry)
                        .chain(iter::once(")"))
                    })
                })
                .collect::<Result<_, _>>()
                .map(Vec::into_iter)
                .map(Iterator::flatten)
                .map(move |rest_of_entries| {
                    iter::once(PAIRS_GROUP_ENTRIES_PREPEND)
                        .chain(first_entry)
                        .chain(rest_of_entries)
                })
        })
        .map(|sources| PairsGroup {
            matcher_parameter_name: "matcher",
            sources,
        })
}

pub(super) struct PairsGroup<I> {
    pub matcher_parameter_name: &'static str,
    pub sources: I,
}

fn entry<'dex_currencies>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    ticker: &str,
) -> Result<impl IntoIterator<Item = &'dex_currencies str>> {
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
