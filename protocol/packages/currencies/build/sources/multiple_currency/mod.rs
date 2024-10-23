use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Context, Result};

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, protocol::Protocol};

use super::{module_and_name::CurrentModule, DexCurrencies};

use self::generate::FinalizedSources;

mod generate;

pub(super) struct SourcesGenerator<
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
    SourcesGenerator<
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
    pub const fn new(
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
            protocol,
            host_currency,
            dex_currencies,
            currencies_tree,
        }
    }
}

impl<'dex_currencies, 'currencies_tree>
    SourcesGenerator<'_, '_, 'dex_currencies, '_, '_, 'currencies_tree, '_, '_, '_, '_>
{
    pub fn generate_and_commit<'ticker, BuildReport, Tickers>(
        &self,
        build_report: BuildReport,
        output_file_path: &Path,
        current_module: CurrentModule,
        tickers: Tickers,
    ) -> Result<()>
    where
        BuildReport: Write,
        Tickers: IntoIterator<Item = &'ticker str>,
    {
        self.generate_sources(current_module, tickers.into_iter())
            .and_then(|sources| Self::commit(build_report, output_file_path, sources))
    }

    fn commit<'r, BuildReport, Sources>(
        mut build_report: BuildReport,
        output_file_path: &Path,
        FinalizedSources {
            currencies_count,
            mut sources,
        }: FinalizedSources<Sources>,
    ) -> Result<()>
    where
        BuildReport: Write,
        Sources: Iterator<Item = Cow<'r, str>>,
    {
        File::create(output_file_path)
            .map(BufWriter::new)
            .context("Failed to open output file for writing!")
            .and_then(|mut output_file| {
                sources
                    .try_for_each(|segment| output_file.write_all(segment.as_bytes()))
                    .and_then(|()| output_file.flush())
                    .with_context(|| {
                        format!("Failed to write generated sources for output file {output_file_path:?}!")
                    })
                    .and_then(|()| {
                        build_report
                            .write_fmt(format_args!(
                                "{output_file_path:?}: {currencies_count} currencies emitted.\n",
                            ))
                            .context("Failed to write build report!")
                    })
            })
    }
}
