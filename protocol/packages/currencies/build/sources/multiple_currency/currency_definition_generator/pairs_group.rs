use std::iter;

use anyhow::Result;

use topology::CurrencyDefinition;

use crate::{
    either::Either,
    protocol::Protocol,
    sources::module_and_name::{CurrentModule, ModuleAndName},
};

use super::DexCurrencies;

pub(super) struct PairsGroup<I> {
    pub matcher_parameter_name: &'static str,
    pub visitor_parameter_name: &'static str,
    pub sources: I,
}

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

    let visit_entry_template = VisitEntryTemplate::new(
        current_module,
        protocol,
        host_currency,
        dex_currencies,
        "visit",
        matcher_parameter_name,
        visitor_parameter_name,
    );

    visit_entry_template
        .apply(ticker)
        .and_then(|first_entry| {
            children
                .map(|ticker| {
                    visit_entry_template.apply(ticker).map(|entry| {
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

struct VisitEntryTemplate<
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> {
    current_module: CurrentModule,
    protocol: &'protocol Protocol,
    host_currency: &'host_currency CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
    visit_function: &'static str,
    matcher_parameter_name: &'static str,
    visitor_parameter_name: &'static str,
}

impl<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
    VisitEntryTemplate<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
{
    const fn new(
        current_module: CurrentModule,
        protocol: &'protocol Protocol,
        host_currency: &'host_currency CurrencyDefinition,
        dex_currencies: &'dex_currencies DexCurrencies<
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
        visit_function: &'static str,
        matcher_parameter_name: &'static str,
        visitor_parameter_name: &'static str,
    ) -> Self {
        Self {
            current_module,
            protocol,
            host_currency,
            dex_currencies,
            visit_function,
            matcher_parameter_name,
            visitor_parameter_name,
        }
    }
}

impl<'dex_currencies> VisitEntryTemplate<'_, '_, 'dex_currencies, '_, '_> {
    fn apply(&self, ticker: &str) -> Result<impl IntoIterator<Item = &'dex_currencies str>> {
        ModuleAndName::resolve(
            self.current_module,
            self.protocol,
            self.host_currency,
            self.dex_currencies,
            ticker,
        )
        .map(|resolved| {
            [
                self.visit_function,
                "::<",
                resolved.module(),
                "::",
                resolved.name(),
                ", _, _>(",
                self.matcher_parameter_name,
                ", ",
                self.visitor_parameter_name,
                ")",
            ]
        })
    }
}
