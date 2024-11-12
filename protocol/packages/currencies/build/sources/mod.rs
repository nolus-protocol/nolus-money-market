use std::{borrow::Borrow, collections::BTreeMap, fs::File, io::Write, iter, path::Path};

use anyhow::{Context as _, Result};

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, protocol::Protocol};

use self::resolved_currency::{CurrentModule, ResolvedCurrency};

mod generator;
mod resolved_currency;
mod writer;

const LPN_NAME: &str = "Lpn";

const NLS_NAME: &str = "Nls";

type DexCurrencies<'ticker, 'currency_definition> =
    BTreeMap<&'ticker str, (String, &'currency_definition CurrencyDefinition)>;

pub(super) fn write<BuildReport>(
    mut build_report: BuildReport,
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &DexCurrencies<'_, '_>,
    currencies_tree: &CurrenciesTree<'_, '_, '_, '_>,
) -> Result<()>
where
    BuildReport: Write,
{
    let generator = writer::Writer::new(currencies_tree);

    let static_context = &generator::StaticContext::new(protocol, host_currency, dex_currencies);

    let builder = generator::Builder::new(static_context);

    write_lease(
        &generator,
        &mut build_report,
        output_directory,
        builder,
        protocol,
    )?;

    write_lpn(
        &generator,
        &mut build_report,
        output_directory,
        builder,
        protocol,
    )?;

    write_native(
        &generator,
        &mut build_report,
        output_directory,
        builder,
        host_currency,
    )?;

    write_payment_only(
        generator,
        &mut build_report,
        output_directory,
        builder,
        protocol,
    )?;

    write_stable(
        build_report,
        output_directory,
        host_currency,
        dex_currencies,
        protocol,
    )
}

#[inline]
fn write_lease<BuildReport>(
    generator: &writer::Writer<'_, '_, '_, '_, '_>,
    build_report: &mut BuildReport,
    output_directory: &Path,
    builder: generator::Builder<'_, '_, '_, '_, '_, '_>,
    protocol: &Protocol,
) -> Result<()>
where
    BuildReport: Write,
{
    generator.generate_and_commit(
        build_report,
        &output_directory.join("lease.rs"),
        &builder.lease(),
        protocol.lease_currencies_tickers.iter().map(Borrow::borrow),
    )
}

#[inline]
fn write_lpn<BuildReport>(
    generator: &writer::Writer<'_, '_, '_, '_, '_>,
    build_report: &mut BuildReport,
    output_directory: &Path,
    builder: generator::Builder<'_, '_, '_, '_, '_, '_>,
    protocol: &Protocol,
) -> Result<()>
where
    BuildReport: Write,
{
    generator.generate_and_commit(
        build_report,
        &output_directory.join("lpn.rs"),
        &builder.lpn(),
        iter::once(&*protocol.lpn_ticker),
    )
}

#[inline]
fn write_native<BuildReport>(
    generator: &writer::Writer<'_, '_, '_, '_, '_>,
    build_report: &mut BuildReport,
    output_directory: &Path,
    builder: generator::Builder<'_, '_, '_, '_, '_, '_>,
    host_currency: &CurrencyDefinition,
) -> Result<()>
where
    BuildReport: Write,
{
    generator.generate_and_commit(
        build_report,
        &output_directory.join("native.rs"),
        &builder.native(),
        iter::once(host_currency.ticker()),
    )
}

#[inline]
fn write_payment_only<BuildReport>(
    generator: writer::Writer<'_, '_, '_, '_, '_>,
    build_report: &mut BuildReport,
    output_directory: &Path,
    builder: generator::Builder<'_, '_, '_, '_, '_, '_>,
    protocol: &Protocol,
) -> Result<()>
where
    BuildReport: Write,
{
    generator.generate_and_commit(
        build_report,
        &output_directory.join("payment_only.rs"),
        &builder.payment_only(),
        protocol
            .payment_only_currencies_tickers
            .iter()
            .map(Borrow::borrow),
    )
}

fn write_stable<Report>(
    mut build_report: Report,
    output_directory: &Path,
    host_currency: &CurrencyDefinition,
    dex_currencies: &DexCurrencies<'_, '_>,
    protocol: &Protocol,
) -> Result<()>
where
    Report: Write,
{
    const FILENAME: &str = "stable.rs";

    let (module, name) = {
        let resolved = ResolvedCurrency::resolve(
            CurrentModule::Stable,
            protocol,
            host_currency,
            dex_currencies,
            &protocol.stable_currency_ticker,
        )?;

        (resolved.module(), resolved.name())
    };

    build_report.write_fmt(format_args!(
        "Stable currency ticker: {} -> {module}::{name}\n",
        protocol.stable_currency_ticker,
    ))?;

    File::create(output_directory.join(FILENAME))
        .with_context(|| format!("Failed to open {FILENAME:?} for writing!"))?
        .write_fmt(format_args!(
            "// @generated

pub type Stable = {module}::{name};
"
        ))
        .with_context(move || format!("Failed to write contents to {FILENAME:?}!"))
}
