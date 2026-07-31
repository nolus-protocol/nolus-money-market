use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use cargo_metadata::Package;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct Config<'r> {
    #[serde(borrow)]
    pub combinations: Vec<Combination<'r>>,
    #[serde(borrow, default)]
    pub feature_groups: BTreeMap<&'r str, FeatureGroup<'r>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct Combination<'r> {
    #[serde(borrow, deserialize_with = "deserialize_btree_set", default)]
    pub tags: BTreeSet<&'r str>,
    #[serde(borrow, deserialize_with = "deserialize_btree_set", default)]
    pub feature_groups: BTreeSet<&'r str>,
    #[serde(borrow, deserialize_with = "deserialize_btree_set", default)]
    pub always_on: BTreeSet<&'r str>,
    pub include_rest: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct FeatureGroup<'r> {
    #[serde(borrow, deserialize_with = "deserialize_btree_set", default)]
    pub members: BTreeSet<&'r str>,
    pub at_least_one: bool,
    pub mutually_exclusive: bool,
}

fn deserialize_btree_set<'r, 'de: 'r, D>(deserializer: D) -> Result<BTreeSet<&'r str>, D::Error>
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
        .map(Config::deserialize)
        .transpose()
        .context("Deserializing configuration failed!")
}
