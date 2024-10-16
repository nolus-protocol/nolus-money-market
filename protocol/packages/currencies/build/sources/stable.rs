use std::{collections::BTreeMap, fs::File, io::Write, path::Path};

use anyhow::{Context as _, Result};

use topology::CurrencyDefinition;

use crate::{protocol::Protocol, LPN_NAME};

const FILENAME: &str = "stable.rs";

pub(super) fn write<Report>(
    mut build_report: Report,
    output_directory: &Path,
    protocol: &Protocol,
    dex_currencies: BTreeMap<&str, (String, &CurrencyDefinition)>,
) -> Result<()>
where
    Report: Write,
{
    let (module, name) = if protocol.stable_currency_ticker == protocol.lpn_ticker {
        ("lpn::impl_mod", LPN_NAME)
    } else {
        let module = if protocol
            .lease_currencies_tickers
            .contains(&protocol.stable_currency_ticker)
        {
            "lease::impl_mod::definitions"
        } else {
            "payment::only::impl_mod::definitions"
        };

        (
            module,
            &*dex_currencies[&*protocol.stable_currency_ticker].0,
        )
    };

    build_report.write_fmt(format_args!(
        "Stable currency ticker: {} -> {module}::{name}\n",
        protocol.stable_currency_ticker,
    ))?;

    File::create(output_directory.join(FILENAME))
        .with_context(|| format!("Failed to open {FILENAME:?} for writing!"))?
        .write_fmt(format_args!(
            "// @generated

pub type Stable = crate::{module}::{name};
"
        ))
        .with_context(move || format!("Failed to write contents to {FILENAME:?}!"))
}
