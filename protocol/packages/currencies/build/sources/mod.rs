use std::{collections::BTreeMap, io::Write, path::Path};

use anyhow::Result;

use topology::CurrencyDefinition;

use crate::{currencies_tree::CurrenciesTree, protocol::Protocol};

use self::module_and_name::CurrentModule;

mod host_native;
mod liquidity_provider_native;
mod module_and_name;
mod multiple_currency;
mod stable;

pub(super) fn write<BuildReport>(
    mut build_report: BuildReport,
    output_directory: &Path,
    protocol: Protocol,
    host_currency: CurrencyDefinition,
    dex_currencies: BTreeMap<&str, (String, &CurrencyDefinition)>,
    currencies_tree: CurrenciesTree<'_, '_, '_, '_>,
) -> Result<()>
where
    BuildReport: Write,
{
    let multiple_currency_source_generator = multiple_currency::SourcesGenerator::new(
        &protocol,
        &host_currency,
        &dex_currencies,
        &currencies_tree,
    );

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("lease.rs"),
        CurrentModule::Lease,
        dex_currencies
            .keys()
            .copied()
            .filter(|&key| protocol.lease_currencies_tickers.contains(key)),
    )?;

    liquidity_provider_native::write(
        &mut build_report,
        output_directory,
        &protocol,
        &host_currency,
        &dex_currencies,
        currencies_tree.children(&protocol.lpn_ticker),
    )?;

    host_native::write(
        &mut build_report,
        output_directory,
        &protocol,
        &host_currency,
        &dex_currencies,
        currencies_tree.parents(host_currency.ticker()),
        currencies_tree.children(host_currency.ticker()),
    )?;

    multiple_currency_source_generator.generate_and_commit(
        &mut build_report,
        &output_directory.join("payment_only.rs"),
        CurrentModule::PaymentOnly,
        dex_currencies.keys().copied().filter(|&key| {
            !(key == protocol.lpn_ticker || protocol.lease_currencies_tickers.contains(key))
        }),
    )?;

    stable::write(build_report, output_directory, &protocol, dex_currencies)
}
