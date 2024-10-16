use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
};

use anyhow::{anyhow, Context as _, Result};

use topology::CurrencyDefinition;

use crate::{protocol::Protocol, NLS_NAME};

use super::module_and_name::ModuleAndName;

pub(super) fn write<W>(
    mut build_report: W,
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    parents: &BTreeSet<&str>,
    children: &BTreeSet<&str>,
) -> Result<()>
where
    W: Write,
{
    let ticker = host_currency.ticker();

    build_report.write_fmt(format_args!("Host currency ticker: {ticker}\n"))?;

    IntoIterator::into_iter([&children, &parents]).try_for_each({
        |paired_with| {
            if paired_with.contains(ticker) {
                Err(anyhow!(
                    "Host's native currency cannot be in a pool with itself!"
                ))
            } else {
                Ok(())
            }
        }
    })?;

    let pairs_group = {
        let mut iter = children.iter();

        if let Some(ticker) = iter.next() {
            let resolved = ModuleAndName::resolve(protocol, host_currency, dex_currencies, ticker)?;

            let mut pairs_group = format!(
                "currency::maybe_visit_buddy::<crate::{module}::{name}, _, _>(matcher, visitor)",
                module = resolved.module(),
                name = resolved.name(),
            );

            for &ticker in iter {
                let resolved =
                    ModuleAndName::resolve(protocol, host_currency, dex_currencies, ticker)?;

                pairs_group.push_str(&format!(
                    "
    .or_else(|visitor| currency::maybe_visit_buddy::<crate::{module}::{name}, _, _>(matcher, visitor))",
                    module = resolved.module(),
                    name = resolved.name(),
                ));
            }

            pairs_group
        } else {
            "currency::visit_noone(visitor)".into()
        }
    };

    let in_pool_with = {
        let mut in_pool_with = String::new();

        for &ticker in parents.iter() {
            let resolved = ModuleAndName::resolve(protocol, host_currency, dex_currencies, ticker)?;

            in_pool_with.push_str(&format!(
                "
impl currency::InPoolWith<crate::{module}::{name}> for {NLS_NAME} {{}}
",
                module = resolved.module(),
                name = resolved.name(),
            ));
        }

        in_pool_with
    };

    fs::write(
        output_directory.join("native.rs"),
        format!(
            r#"// @generated

use serde::{{Deserialize, Serialize}};

use currency::{{
CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
PairsVisitor,
}};
use sdk::schemars::JsonSchema;

use crate::payment;

#[derive(
Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct {NLS_NAME}(CurrencyDTO<super::Group>);

impl CurrencyDef for {NLS_NAME} {{
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

impl PairsGroup for {NLS_NAME} {{
type CommonGroup = payment::Group;

fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
where
M: Matcher,
V: PairsVisitor<Pivot = Self>,
{{
{pairs_group}
}}
}}
{in_pool_with}"#,
            host_path = host_currency.host().path(),
            host_symbol = host_currency.host().symbol(),
            dex_path = host_currency.dex().path(),
            dex_symbol = host_currency.dex().symbol(),
            decimals = host_currency.decimal_digits(),
        ),
    )
    .context("Failed to write host's native currency implementation!")
}
