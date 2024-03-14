use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Context, Result};
use cargo_metadata::Package;
use serde::{Deserialize, Deserializer};

#[derive(Debug)]
pub(crate) struct Config<'r> {
    pub combinations: Vec<Combination<'r>>,
    pub sets: BTreeMap<&'r str, Set<'r>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", untagged, deny_unknown_fields)]
pub(crate) enum IdentOrList<'r> {
    Ident(#[serde(borrow)] &'r str),
    List(#[serde(borrow, deserialize_with = "deserialize_set")] BTreeSet<&'r str>),
}

#[derive(Debug)]
pub(crate) struct Combination<'r> {
    pub groups: BTreeSet<&'r str>,
    pub sets: BTreeSet<&'r str>,
    pub always_on: BTreeSet<&'r str>,
    pub include_rest: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct Set<'r> {
    #[serde(borrow, deserialize_with = "deserialize_set", default)]
    pub members: BTreeSet<&'r str>,
    pub at_least_one: bool,
    pub mutually_exclusive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct GenericConfig<'r> {
    #[serde(borrow)]
    combinations: Vec<GenericCombination<'r>>,
    #[serde(borrow, default)]
    sets: BTreeMap<&'r str, Set<'r>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct GenericCombination<'r> {
    #[serde(borrow, default)]
    generics: BTreeMap<&'r str, IdentOrList<'r>>,
    #[serde(borrow, deserialize_with = "deserialize_set", default)]
    groups: BTreeSet<&'r str>,
    #[serde(borrow, deserialize_with = "deserialize_set", default)]
    sets: BTreeSet<&'r str>,
    #[serde(borrow, deserialize_with = "deserialize_set", default)]
    always_on: BTreeSet<&'r str>,
    include_rest: bool,
}

fn deserialize_set<'r, 'de: 'r, D>(deserializer: D) -> Result<BTreeSet<&'r str>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<&'r str>::deserialize(deserializer)
        .map(Vec::into_iter)
        .map(BTreeSet::from_iter)
}

pub(crate) fn deserialize_config_if_any(package: &Package) -> Result<Option<Config<'_>>> {
    package
        .metadata
        .get("cargo-each")
        .map(GenericConfig::deserialize)
        .transpose()
        .context("Deserializing configuration failed!")
        .and_then(|maybe_config| {
            maybe_config
                .map(|config| {
                    try_resolve_config_generics(package, config)
                        .context("Failed to resolve configurations' generics!")
                })
                .transpose()
        })
}

fn try_resolve_config_generics<'r>(
    package: &'r Package,
    config: GenericConfig<'r>,
) -> Result<Config<'r>> {
    let mut combinations = collect_non_generic_combinations(&config);

    config
        .combinations
        .into_iter()
        .filter(|combination| !combination.generics.is_empty())
        .try_for_each(
            |GenericCombination {
                 generics,
                 groups,
                 sets,
                 always_on,
                 include_rest,
             }| {
                construct_generic_mappings(package, &config.sets, generics)
                    .context("Error occurred while constructing generic parameter mappings!")
                    .map(|generics_mappings| {
                        combinations.extend(generics_mappings.into_iter().map(|replacements| {
                            replace_generics(
                                &groups,
                                &sets,
                                &always_on,
                                include_rest,
                                &replacements,
                            )
                        }));
                    })
            },
        )
        .map(|()| Config {
            combinations,
            sets: config.sets,
        })
}

fn collect_non_generic_combinations<'r>(config: &GenericConfig<'r>) -> Vec<Combination<'r>> {
    config
        .combinations
        .iter()
        .filter(|combination| combination.generics.is_empty())
        .map(
            |&GenericCombination {
                 generics: _,
                 ref groups,
                 ref sets,
                 ref always_on,
                 include_rest,
             }| Combination {
                groups: groups.clone(),
                sets: sets.clone(),
                always_on: always_on.clone(),
                include_rest,
            },
        )
        .collect()
}

fn construct_generic_mappings<'r>(
    package: &'r Package,
    sets: &BTreeMap<&'r str, Set<'r>>,
    generics: BTreeMap<&'r str, IdentOrList<'r>>,
) -> Result<Vec<BTreeMap<&'r str, &'r str>>> {
    let mut generics_mappings: Vec<BTreeMap<&str, &str>> = Vec::new();

    for (placeholder, ref replacements) in generics {
        let generics = match replacements {
            IdentOrList::Ident(set) => {
                sets.get(set).map(|set| &set.members).ok_or_else(
                    #[cold]
                    || anyhow!(r#"Combination is generic over set "{set}", but package "{}" doesn't define such!"#, package.name)
                )?
            }
            IdentOrList::List(replacements) => replacements,
        };

        generics_mappings = generics_mappings
            .is_empty()
            .then(BTreeMap::new)
            .into_iter()
            .chain(generics_mappings.into_iter())
            .flat_map(move |generics_mapping| {
                generics.iter().map(move |&replacement| {
                    let mut generics_mapping = generics_mapping.clone();

                    generics_mapping.insert(placeholder, replacement);

                    generics_mapping
                })
            })
            .collect();
    }

    Ok(generics_mappings)
}

fn replace_generics<'r>(
    groups: &BTreeSet<&'r str>,
    sets: &BTreeSet<&'r str>,
    always_on: &BTreeSet<&'r str>,
    include_rest: bool,
    replacements: &BTreeMap<&'r str, &'r str>,
) -> Combination<'r> {
    Combination {
        groups: groups
            .iter()
            .map(|&group| replacements.get(group).copied().unwrap_or(group))
            .collect(),
        sets: sets
            .iter()
            .map(|&set| replacements.get(set).copied().unwrap_or(set))
            .collect(),
        always_on: always_on
            .iter()
            .map(|&always_on| replacements.get(always_on).copied().unwrap_or(always_on))
            .collect(),
        include_rest,
    }
}
