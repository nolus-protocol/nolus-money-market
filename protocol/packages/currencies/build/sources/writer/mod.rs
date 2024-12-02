use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Context, Result};

use crate::currencies_tree::CurrenciesTree;

use super::generator;

mod generate;

pub(super) struct Writer<'currencies_tree, 'parents_of, 'parent, 'children_of, 'child> {
    currencies_tree: &'currencies_tree CurrenciesTree<'parents_of, 'parent, 'children_of, 'child>,
}

impl<'currencies_tree, 'parents_of, 'parent, 'children_of, 'child>
    Writer<'currencies_tree, 'parents_of, 'parent, 'children_of, 'child>
{
    #[inline]
    pub const fn new(
        currencies_tree: &'currencies_tree CurrenciesTree<
            'parents_of,
            'parent,
            'children_of,
            'child,
        >,
    ) -> Self {
        Self { currencies_tree }
    }
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition> Writer<'_, '_, '_, '_, '_>
where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    #[inline]
    pub fn generate_and_commit<'ticker, BuildReport, Generator, Tickers>(
        &self,
        build_report: BuildReport,
        output_file_path: &Path,
        generator: &Generator,
        tickers: Tickers,
    ) -> Result<()>
    where
        BuildReport: Write,
        Generator: generator::Resolver<'dex_currencies, 'dex_currencies>
            + generator::MaybeVisit
            + generator::PairsGroup<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
            + generator::InPoolWith<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>,
        Tickers: IntoIterator<Item = &'ticker str>,
    {
        self.generate_sources(generator, tickers.into_iter())
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

struct FinalizedSources<Sources> {
    currencies_count: usize,
    sources: Sources,
}
