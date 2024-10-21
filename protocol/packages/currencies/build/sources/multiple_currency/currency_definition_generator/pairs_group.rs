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
    visitor_parameter_name: &'static str,
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
            "matcher",
            visitor_parameter_name,
            children,
        )
        .map(
            |PairsGroup {
                 matcher_parameter_name,
                 visitor_parameter_name,
                 sources,
             }| PairsGroup {
                matcher_parameter_name,
                visitor_parameter_name,
                sources: Either::Left(sources),
            },
        )
    } else {
        Ok(PairsGroup {
            matcher_parameter_name: "_",
            visitor_parameter_name,
            sources: Either::Right(
                ["currency::visit_noone(", visitor_parameter_name, ")"].into_iter(),
            ),
        })
    }
}

fn non_empty_pairs_group<'dex_currencies, 'child, Children>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    ticker: &str,
    matcher_parameter_name: &'static str,
    visitor_parameter_name: &'static str,
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
            "visit",
            matcher_parameter_name,
            visitor_parameter_name,
            ticker,
        )
    };

    process_ticker(ticker)
        .and_then(|first_entry| {
            children
                .map(|ticker| {
                    process_ticker(ticker).map(|entry| {
                        [
                            "
                .or_else(|",
                            visitor_parameter_name,
                            "| ",
                        ]
                        .into_iter()
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
            matcher_parameter_name,
            visitor_parameter_name,
            sources,
        })
}

pub(super) struct PairsGroup<I> {
    pub matcher_parameter_name: &'static str,
    pub visitor_parameter_name: &'static str,
    pub sources: I,
}

fn entry<'dex_currencies>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    visit_function: &'static str,
    matcher_parameter_name: &'static str,
    visitor_parameter_name: &'static str,
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
            visit_function,
            "::<",
            resolved.module(),
            "::",
            resolved.name(),
            ", _, _>(",
            matcher_parameter_name,
            ", ",
            visitor_parameter_name,
            ")",
        ]
    })
}
