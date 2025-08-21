use std::{env, ffi::OsStr, fs::File, io::Write, path::Path};

use anyhow::{Context as _, Result, anyhow, bail};

use topology::{CurrencyDefinitions, Topology};

use self::{currencies_tree::CurrenciesTree, protocol::Protocol};

mod convert_case;
mod currencies_tree;
mod protocol;
mod sources;
mod subtype_lifetime;
mod swap_pairs;

const PROTOCOL_JSON: &str = "../../../build-configuration/protocol.json";

const TOPOLOGY_JSON: &str = "../../../build-configuration/topology.json";

const BUILD_OUT_DIR_PATHS: &str = "BUILD_OUT_DIR_PATHS";

fn main() -> Result<()> {
    for path in ["build/", PROTOCOL_JSON, TOPOLOGY_JSON] {
        println!("cargo::rerun-if-changed={path}");
    }

    println!("cargo::rerun-if-env-changed={BUILD_OUT_DIR_PATHS}");

    if env::var_os("CARGO_FEATURE_TESTING").is_some() {
        Ok(())
    } else if check_for_definitions()? {
        let output_directory =
            env::var_os("OUT_DIR").context("Cargo did not set `OUT_DIR` environment variable!")?;

        let build_report = build_report_writer(output_directory.as_ref())?;

        let output_directory = {
            if let Some(build_dir_paths) = env::var_os(BUILD_OUT_DIR_PATHS) {
                let mut build_dir_paths = File::options()
                    .append(true)
                    .create(true)
                    .open(build_dir_paths)?;

                build_dir_paths.write_all(output_directory.as_encoded_bytes())?;

                build_dir_paths.write_all(AsRef::<OsStr>::as_ref("\n").as_encoded_bytes())?;

                build_dir_paths.flush()?;
            }

            AsRef::as_ref(&output_directory)
        };

        let topology = File::open(TOPOLOGY_JSON)
            .context(r#"Failed to open "topology.json"!"#)
            .and_then(|file| {
                serde_json::from_reader(file).context("Failed to parse topology JSON!")
            })?;

        let protocol = File::open(PROTOCOL_JSON)
            .context(r#"Failed to open "protocol.json"!"#)
            .and_then(|file| {
                serde_json::from_reader(file).context("Failed to parse protocol JSON!")
            })?;

        generate_currencies(build_report, output_directory, topology, protocol)
    } else {
        Err(anyhow!(
            "Topology and protocol definitions don't exist while `tesing` \
            feature is not selected!"
        ))
    }
}

fn check_for_definitions() -> Result<bool> {
    IntoIterator::into_iter([PROTOCOL_JSON, TOPOLOGY_JSON])
        .map(Path::new)
        .map(Path::try_exists)
        .try_fold(true, |all_exist, result| {
            result
                .map(|exists| all_exist && exists)
                .context("Failed to check whether JSON descriptor file exists!")
        })
}

fn build_report_writer(output_directory: &Path) -> Result<impl Write + use<>> {
    File::create(output_directory.join("currencies-build.log"))
        .context("Failed to open build report for writing!")
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

    if protocol.lpn_ticker == host_currency.ticker() {
        bail!(
            "Liquidity provider's currency cannot be the same as the host \
                network's native currency!",
        );
    }

    if protocol.stable_currency_ticker == host_currency.ticker() {
        bail!(
            "Stable currency cannot be the same as the host network's native \
                currency!",
        );
    }

    sources::write(
        build_report,
        output_directory,
        &protocol,
        &host_currency,
        &protocol.dex_currencies(&host_currency, &dex_currencies),
        &CurrenciesTree::new(&protocol, &host_currency)?,
    )
}
