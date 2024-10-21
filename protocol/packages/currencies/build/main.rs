use std::{
    env,
    fs::File,
    io::{self, Write},
    iter,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context as _, Result};

use topology::{CurrencyDefinitions, Topology};

use self::{currencies_tree::CurrenciesTree, either::Either, protocol::Protocol};

mod currencies_tree;
mod either;
mod protocol;
mod sources;
mod subtype_lifetime;

const PROTOCOL_JSON: &str = "./../../../build-configuration/protocol.json";

const TOPOLOGY_JSON: &str = "./../../../build-configuration/topology.json";

const LPN_NAME: &str = "Lpn";

const NLS_NAME: &str = "Nls";

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

    let currencies_tree = CurrenciesTree::new(&topology, &protocol)?;

    sources::write(
        build_report,
        output_directory,
        protocol,
        host_currency,
        dex_currencies
            .iter()
            .map(|currency_definition| {
                (
                    currency_definition.ticker(),
                    (
                        snake_case_to_upper_camel_case(currency_definition.ticker()),
                        currency_definition,
                    ),
                )
            })
            .collect(),
        currencies_tree,
    )
}

fn snake_case_to_upper_camel_case(mut input: &str) -> String {
    let mut string = String::new();

    iter::from_fn(move || {
        input
            .find('_')
            .or_else(|| (!input.is_empty()).then_some(input.len()))
            .map(|index| {
                let substring = &input[..index];

                input = input.get(index + 1..).unwrap_or("");

                substring
            })
    })
    .for_each(|substring| {
        let mut chars = substring.chars();

        if let Some(first_character) = chars.next() {
            string.push(first_character.to_ascii_uppercase());

            chars
                .map(|ch| ch.to_ascii_lowercase())
                .for_each(|ch| string.push(ch));
        }
    });

    string
}
