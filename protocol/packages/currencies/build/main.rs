use std::{
    env,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context as _, Result};

use topology::{CurrencyDefinitions, Topology};

use self::{currencies_tree::CurrenciesTree, either::Either, protocol::Protocol};

mod convert_case;
mod currencies_tree;
mod either;
mod protocol;
mod sources;
mod subtype_lifetime;

const PROTOCOL_JSON: &str = "../../../build-configuration/protocol.json";

const TOPOLOGY_JSON: &str = "../../../build-configuration/topology.json";

const BUILD_REPORT: &str = "CURRENCIES_BUILD_REPORT";

fn main() -> Result<()> {
    for path in ["build/", PROTOCOL_JSON, TOPOLOGY_JSON] {
        println!("cargo::rerun-if-changed={path}");
    }

    println!("cargo::rerun-if-env-changed={BUILD_REPORT}");

    let output_directory: &Path = &PathBuf::from(
        env::var_os("OUT_DIR").context("Cargo did not set `OUT_DIR` environment variable!")?,
    );

    let files_exist = IntoIterator::into_iter([PROTOCOL_JSON, TOPOLOGY_JSON])
        .map(Path::new)
        .map(Path::try_exists)
        .try_fold(true, |all_exist, result| {
            result
                .map(|exists| all_exist && exists)
                .context("Failed to check whether JSON descriptor file exists!")
        })?;

    let build_report = if let Some(report_file) = env::var_os(BUILD_REPORT) {
        if files_exist {
            Either::Left(
                File::create(report_file).context("Failed to open build report for writing!")?,
            )
        } else {
            bail!(
                "`{BUILD_REPORT:?}` environment variable set but topology \
                and/or protocol descriptors don't exist!",
            );
        }
    } else {
        Either::Right(io::stderr())
    };

    if files_exist {
        generate_currencies(
            build_report,
            output_directory,
            serde_json::from_reader(
                File::open(TOPOLOGY_JSON).context("Failed to open \"topology.json\"!")?,
            )
            .context("Failed to parse topology JSON!")?,
            serde_json::from_reader(
                File::open(PROTOCOL_JSON).context("Failed to open \"protocol.json\"!")?,
            )
            .context("Failed to parse protocol JSON!")?,
        )
    } else {
        Ok(())
    }
}

fn generate_currencies<BuildReport>(
    build_report: BuildReport,
    output_directory: &Path,
    topology: Topology,
    protocol: Protocol,
) -> Result<()>
where
    BuildReport: Write,
{
    let CurrencyDefinitions {
        host_currency,
        dex_currencies,
    } = topology.currency_definitions(&protocol.dex_network)?;

    if *protocol.lpn_ticker == *host_currency.ticker() {
        bail!(
            "Liquidity provider's currency cannot be the same as the host \
                network's native currency!",
        );
    }

    if *protocol.stable_currency_ticker == *host_currency.ticker() {
        bail!(
            "Stable currency cannot be the same as the host network's native \
                currency!",
        );
    }

    let dex_currencies = dex_currencies
        .iter()
        .filter(|currency_definition| {
            filter_selected_currencies(
                &protocol,
                host_currency.ticker(),
                currency_definition.ticker(),
            )
        })
        .map(|currency_definition| {
            (
                currency_definition.ticker(),
                (
                    convert_case::snake_case_to_upper_camel_case(currency_definition.ticker()),
                    currency_definition,
                ),
            )
        })
        .collect();

    sources::write(
        build_report,
        output_directory,
        &protocol,
        &host_currency,
        &dex_currencies,
        &CurrenciesTree::new(&topology, &protocol, host_currency.ticker())?,
    )
}

#[inline]
fn filter_selected_currencies(
    protocol: &Protocol,
    host_currency_ticker: &str,
    ticker: &str,
) -> bool {
    ticker == host_currency_ticker
        || ticker == protocol.lpn_ticker
        || protocol.lease_currencies_tickers.contains(ticker)
        || protocol.payment_only_currencies_tickers.contains(ticker)
}
