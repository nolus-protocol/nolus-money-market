use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
};

use anyhow::{bail, Context, Result};

use topology::CurrencyDefinition;

use crate::{protocol::Protocol, LPN_NAME};

use super::module_and_name::{CurrentModule, ModuleAndName};

pub(super) fn write<W>(
    mut build_report: W,
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    children: &BTreeSet<&str>,
) -> Result<()>
where
    W: Write,
{
    let lpn = dex_currencies
    .get(&*protocol.lpn_ticker)
    .context("Selected DEX network doesn't define such currency provided as liquidity provider's native currency!")?
    .1;

    let ticker = lpn.ticker();

    build_report.write_fmt(format_args!("LPN ticker: {ticker}\n"))?;

    let in_pool_with = {
        if children.contains(ticker) {
            bail!(
                "Liquidity provider's native currency cannot be in a pool with \
            itself!",
            );
        }

        let mut in_pool_with = String::new();

        for &ticker in children.iter() {
            let resolved = ModuleAndName::resolve(
                protocol,
                host_currency,
                dex_currencies,
                ticker,
                CurrentModule::Lpn,
            )?;

            in_pool_with.push_str(&format!(
                "
impl currency::InPoolWith<crate::{module}::{name}> for {LPN_NAME} {{}}
",
                module = resolved.module(),
                name = resolved.name(),
            ));
        }

        in_pool_with
    };

    fs::write(
        output_directory.join("lpn.rs"),
        format!(
            r#"// @generated

use serde::{{Deserialize, Serialize}};

use currency::{{CurrencyDTO, CurrencyDef, Definition}};
use sdk::schemars::JsonSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct {LPN_NAME}(CurrencyDTO<super::Group>);

impl CurrencyDef for {LPN_NAME} {{
    type Group = super::Group;

    fn definition() -> &'static Self {{
        const {{
            &Self(CurrencyDTO::new(
                const {{
                    &Definition::new(
                        {ticker:?},
                        // {host_path}
                        {host_symbol:?},
                        // {dex_path}
                        {dex_symbol:?},
                        {decimals},
                    )
                }},
            ))
        }}
    }}

    fn dto(&self) -> &CurrencyDTO<Self::Group> {{
        &self.0
    }}
}}
{in_pool_with}"#,
            host_path = lpn.host().path(),
            host_symbol = lpn.host().symbol(),
            dex_path = lpn.dex().path(),
            dex_symbol = lpn.dex().symbol(),
            decimals = lpn.decimal_digits(),
        ),
    )
    .context("Failed to write liquidity provider's native implementation!")
}
