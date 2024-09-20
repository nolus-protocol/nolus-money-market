use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    io::Write as _,
    iter,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context as _, Result};

use topology::{CurrencyDefinition, CurrencyDefinitions, Topology};

use self::protocol::Protocol;

mod protocol;

const PROTOCOL_JSON: &str = "./../../../build-configuration/protocol.json";

const TOPOLOGY_JSON: &str = "./../../../build-configuration/topology.json";

const LPN_NAME: &str = "Lpn";

const NLS_NAME: &str = "Nls";

fn main() -> Result<()> {
    for path in ["build.rs", PROTOCOL_JSON, TOPOLOGY_JSON] {
        println!("cargo::rerun-if-changed={path}");
    }

    let output_directory: &Path = &PathBuf::from(
        env::var_os("OUT_DIR").context("Cargo did not set `OUT_DIR` environment variable!")?,
    );

    if IntoIterator::into_iter([PROTOCOL_JSON, TOPOLOGY_JSON])
        .map(Path::new)
        .map(Path::try_exists)
        .try_fold(true, |all_exist, result| {
            result
                .map(|exists| all_exist && exists)
                .context("Failed to check whether JSON descriptor file exists!")
        })?
    {
        generate_currencies(
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

fn generate_currencies(
    output_directory: &Path,
    topology: Topology,
    protocol: Protocol,
) -> Result<()> {
    let CurrencyDefinitions {
        host_currency,
        dex_currencies,
    } = topology.currency_definitions(&protocol.dex_network)?;

    if *protocol.liquidity_provider_currency_ticker == *host_currency.ticker() {
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

    let dex_currencies: BTreeMap<_, _> = dex_currencies
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
        .collect();

    let [parents, children] = topology
        .network_dexes(&protocol.dex_network)
        .context("Selected DEX network doesn't define any DEXes!")?
        .get(&protocol.dex)
        .context("Selected DEX network doesn't define such DEX!")?
        .swap_pairs()
        .iter()
        .try_fold(
            [const { BTreeMap::<_, BTreeSet<_>>::new() }; 2],
            |[mut parents, mut children], (from, targets)| {
                if children
                    .insert(from.as_ref(), targets.iter().map(AsRef::as_ref).collect())
                    .is_some()
                {
                    // TODO
                    bail!("TODO");
                }

                targets
                    .iter()
                    .map(AsRef::as_ref)
                    .try_for_each(|target| {
                        if parents.entry(target).or_default().insert(from.as_ref()) {
                            Ok(())
                        } else {
                            // TODO
                            Err(anyhow!("TODO"))
                        }
                    })
                    .map(|()| [parents, children])
            },
        )?;

    let [parents, children] = [&parents, &children].map(move |set| {
        move |currency: &str| set.get(currency).unwrap_or(const { &BTreeSet::new() })
    });

    new_write_multiple_currencies_source(
        &output_directory.join("lease.rs"),
        &protocol,
        &host_currency,
        &dex_currencies,
        dex_currencies
            .keys()
            .copied()
            .filter(|&key| protocol.lease_currencies_tickers.contains(key)),
        &parents,
        &children,
    )?;

    new_write_liquidity_provider_native_source(
        output_directory,
        &protocol,
        &host_currency,
        &dex_currencies,
        children(&protocol.liquidity_provider_currency_ticker),
    )?;

    new_write_host_native_source(
        output_directory,
        &protocol,
        &host_currency,
        &dex_currencies,
        parents(host_currency.ticker()),
        children(host_currency.ticker()),
    )?;

    new_write_multiple_currencies_source(
        &output_directory.join("payment_only.rs"),
        &protocol,
        &host_currency,
        &dex_currencies,
        dex_currencies.keys().copied().filter(|&key| {
            !(key == protocol.liquidity_provider_currency_ticker
                || protocol.lease_currencies_tickers.contains(key))
        }),
        parents,
        children,
    )?;

    write_stable_source(output_directory, &protocol, dex_currencies)
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

fn new_write_multiple_currencies_source<
    'ticker,
    'parent_map,
    'parent,
    'child_map,
    'child,
    CurrenciesIter,
    ParentsF,
    ChildrenF,
>(
    output_file: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    mut currencies: CurrenciesIter,
    mut parents: ParentsF,
    mut children: ChildrenF,
) -> Result<()>
where
    'ticker: 'parent_map + 'child_map,
    'parent: 'parent_map,
    'child: 'child_map,
    CurrenciesIter: Iterator<Item = &'ticker str> + Clone,
    ParentsF: FnMut(&'ticker str) -> &'parent_map BTreeSet<&'parent str>,
    ChildrenF: FnMut(&'ticker str) -> &'child_map BTreeSet<&'child str>,
{
    let maybe_visit_body = {
        let mut currencies = currencies.clone();

        if let Some(ticker) = currencies.next() {
            let process_ticker = |source: &mut String, ticker: &str| -> Result<()> {
                // TODO
                let name = &dex_currencies.get(ticker).context("TODO")?.0;

                source.push_str("maybe_visit_member::<_, definitions::");
                source.push_str(name);
                source.push_str(", VisitedG, _>(matcher, visitor)");

                Ok(())
            };

            let mut source = "use currency::maybe_visit_member;

    "
            .into();

            process_ticker(&mut source, ticker)?;

            for ticker in currencies {
                source.push_str(
                    "
        .or_else(|visitor| ",
                );
                process_ticker(&mut source, ticker)?;
                source.push(')');
            }

            Cow::Owned(source)
        } else {
            Cow::Borrowed("currency::visit_noone(visitor)")
        }
    };

    let currencies = {
        let mut source = String::new();

        if let Some(ticker) = currencies.next() {
            source.push_str(
                "
pub(crate) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, Matcher, MaybePairsVisitorResult, PairsGroup,
        PairsVisitor,
    };
    use sdk::schemars::JsonSchema;

    use crate::payment;
",
            );

            let mut process_ticker = |ticker| -> Result<()> {
                let parents = parents(ticker);

                let children = children(ticker);

                // TODO
                let (name, currency) = dex_currencies.get(ticker).context("TODO")?;

                IntoIterator::into_iter([children, parents]).try_for_each({
                    |paired_with| {
                        if paired_with.contains(ticker) {
                            Err(anyhow!("Currency cannot be in a pool with itself!"))
                        } else {
                            Ok(())
                        }
                    }
                })?;

                let pairs_group = {
                    let mut iter = children.iter();

                    if let Some(ticker) = iter.next() {
                        let process_ticker = |source: &mut String, ticker: &str| {
                            resolve_module_and_name(protocol, host_currency, dex_currencies, ticker)
                                .map(|ModuleAndName { module, name }| {
                                    source.push_str("currency::maybe_visit_buddy::<crate::");

                                    source.push_str(module);

                                    source.push_str("::");

                                    source.push_str(name);

                                    source.push_str(", _, _>(matcher, visitor)");
                                })
                        };

                        let mut source = String::new();

                        process_ticker(&mut source, ticker)?;

                        for &ticker in iter {
                            source.push_str(
                                "
                .or_else(|visitor| ",
                            );
                            process_ticker(&mut source, ticker)?;
                            source.push(')');
                        }

                        Cow::Owned(source)
                    } else {
                        Cow::Borrowed("currency::visit_noone(visitor)")
                    }
                };

                let in_pool_with = {
                    let mut in_pool_with = String::new();

                    for &ticker in parents.iter() {
                        let ModuleAndName {
                            module,
                            name: paired_with,
                        } = resolve_module_and_name(
                            protocol,
                            host_currency,
                            dex_currencies,
                            ticker,
                        )?;

                        in_pool_with.push_str(&format!(
                            "
    impl currency::InPoolWith<crate::{module}::{paired_with}> for {name} {{}}
",
                        ));
                    }

                    in_pool_with
                };

                source.push_str(&format!(
                    r#"
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    #[schemars(crate = "sdk::schemars")]
    pub struct {name}(CurrencyDTO<super::super::Group>);

    impl CurrencyDef for {name} {{
        type Group = super::super::Group;

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

    impl PairsGroup for {name} {{
        type CommonGroup = payment::Group;

        fn maybe_visit<M, V>({matcher}: &M, visitor: V) -> MaybePairsVisitorResult<V>
        where
            M: Matcher,
            V: PairsVisitor<Pivot = Self>,
        {{
            {pairs_group}
        }}
    }}
{in_pool_with}"#,
                    host_path = currency.host().path(),
                    host_symbol = currency.host().symbol(),
                    dex_path = currency.dex().path(),
                    dex_symbol = currency.dex().symbol(),
                    decimals = currency.decimal_digits(),
                    matcher = if matches!(pairs_group, Cow::Borrowed(_)) {
                        "_"
                    } else {
                        "matcher"
                    },
                ));

                Ok(())
            };

            process_ticker(ticker)?;

            currencies.try_for_each(process_ticker)?;

            source.push_str(
                "}
",
            );
        }

        source
    };

    fs::write(
        output_file,
        format!(
            r#"// @generated

use currency::{{AnyVisitor, Group, Matcher, MaybeAnyVisitResult, MemberOf}};

use crate::payment;

pub(super) fn maybe_visit<M, V, VisitedG>(
    {matcher}: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    super::Group: MemberOf<VisitedG>,
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = payment::Group>,
{{
    {maybe_visit_body}
}}
{currencies}"#,
            matcher = if matches!(maybe_visit_body, Cow::Borrowed(_)) {
                "_"
            } else {
                "matcher"
            }
        ),
    )
    .context("Failed to write host's native currency implementation!")
}

fn new_write_liquidity_provider_native_source(
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    children: &BTreeSet<&str>,
) -> Result<()> {
    let lpn = dex_currencies
        .get(&*protocol.liquidity_provider_currency_ticker)
        .context("Selected DEX network doesn't define such currency provided as liquidity provider's native currency!")?
        .1;

    let ticker = lpn.ticker();

    let host_path = lpn.host().path();

    let host_symbol = lpn.host().symbol();

    let dex_path = lpn.dex().path();

    let dex_symbol = lpn.dex().symbol();

    let decimals = lpn.decimal_digits();

    let in_pool_with = {
        if children.contains(ticker) {
            bail!("Liquidity provider's native currency cannot be in a pool with itself!");
        }

        let mut in_pool_with = String::new();

        for &ticker in children.iter() {
            let ModuleAndName { module, name } =
                resolve_module_and_name(protocol, host_currency, dex_currencies, ticker)?;

            in_pool_with.push_str(&format!(
                "
impl currency::InPoolWith<crate::{module}::{name}> for {LPN_NAME} {{}}
",
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
{in_pool_with}"#
        ),
    )
    .context("Failed to write liquidity provider's native implementation!")
}

fn new_write_host_native_source(
    output_directory: &Path,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &BTreeMap<&str, (String, &CurrencyDefinition)>,
    parents: &BTreeSet<&str>,
    children: &BTreeSet<&str>,
) -> Result<()> {
    let ticker = host_currency.ticker();

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
            let ModuleAndName { module, name } =
                resolve_module_and_name(protocol, host_currency, dex_currencies, ticker)?;

            let mut pairs_group = format!(
                "currency::maybe_visit_buddy::<crate::{module}::{name}, _, _>(matcher, visitor)"
            );

            for &ticker in iter {
                let ModuleAndName { module, name } =
                    resolve_module_and_name(protocol, host_currency, dex_currencies, ticker)?;

                pairs_group.push_str(&format!(
                    "
            .or_else(|visitor| currency::maybe_visit_buddy::<crate::{module}::{name}, _, _>(matcher, visitor))",
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
            let ModuleAndName { module, name } =
                resolve_module_and_name(protocol, host_currency, dex_currencies, ticker)?;

            in_pool_with.push_str(&format!(
                "
impl currency::InPoolWith<crate::{module}::{name}> for {NLS_NAME} {{}}
",
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

fn resolve_module_and_name<'r>(
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'r BTreeMap<&str, (String, &CurrencyDefinition)>,
    ticker: &str,
) -> Result<ModuleAndName<'static, 'r>> {
    if let Some((name, _)) = dex_currencies.get(ticker) {
        Ok(if ticker == protocol.liquidity_provider_currency_ticker {
            ModuleAndName {
                module: "lpn::impl_mod",
                name: LPN_NAME,
            }
        } else {
            ModuleAndName {
                module: if protocol.lease_currencies_tickers.contains(ticker) {
                    "lease::impl_mod::definitions"
                } else {
                    "payment::only::impl_mod::definitions"
                },
                name: name.as_str(),
            }
        })
    } else if ticker == host_currency.ticker() {
        Ok(ModuleAndName {
            module: "native",
            name: NLS_NAME,
        })
    } else {
        // TODO
        Err(anyhow!("TODO"))
    }
}

struct ModuleAndName<'module, 'name> {
    module: &'module str,
    name: &'name str,
}

fn write_stable_source(
    output_directory: &Path,
    protocol: &Protocol,
    dex_currencies: BTreeMap<&str, (String, &CurrencyDefinition)>,
) -> Result<()> {
    let (module, name) =
        if protocol.stable_currency_ticker == protocol.liquidity_provider_currency_ticker {
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

    file_writer(output_directory, "stable.rs")?(&format!(
        "pub type Stable = crate::{module}::{name};"
    ))
}

fn file_writer(
    output_directory: &Path,
    filename: &'static str,
) -> Result<impl FnMut(&str) -> Result<()>> {
    File::create(output_directory.join(filename))
        .with_context(|| format!("Failed to open {filename:?} for writing!"))
        .map(|mut file| {
            move |content: &str| -> Result<()> {
                file.write_all(content.as_bytes())
                    .with_context(move || format!("Failed to write contents to {filename:?}!"))
            }
        })
}
