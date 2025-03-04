use anyhow::{Result, anyhow};
use cargo_metadata::Package;

use crate::config::Config;

pub(crate) fn configuration(package: &Package, config: &Config<'_>) -> Result<()> {
    sets(package, config).and_then(|()| combinations(package, config))
}

fn sets(package: &Package, config: &Config<'_>) -> Result<()> {
    config.feature_groups.iter().try_for_each(|(name, set)| {
        if set.members.is_empty() {
            Err(anyhow!(
                r#"Package "{}"'s configuration's set "{}" is empty!"#,
                package.name,
                name,
            ))
        } else if set.at_least_one && set.members.len() == 1 {
            Err(anyhow!(
                r#"Package "{}"'s configuration's set "{}" contains only one item and is marked as `at_least_one`! Consider moving it to a `always-on` section."#,
                package.name,
                name,
            ))
        } else if let Some(feature) = set.members.iter()
            .find(|&&feature| !(
                feature.contains('/') ||
                    feature.starts_with("dep:") ||
                    package.features.contains_key(feature)
            )) {
            Err(anyhow!(
                r#"Package "{}"'s configuration's set "{}" contains an undefined feature "{}"!"#,
                package.name,
                name,
                feature,
            ))
        } else { Ok(()) }
    })
}

fn combinations(package: &Package, config: &Config<'_>) -> Result<()> {
    config.combinations.iter().try_for_each(|combination| {
        if let Some(set) = combination.feature_groups.iter().find(|&set| !config.feature_groups.contains_key(set)) {
            Err(anyhow!(
                r#"Package "{}"'s configuration contains combinations referring to undefined set "{}"!"#,
                package.name,
                set,
            ))
        } else if let Some(set) = combination.feature_groups.iter().map(|set| (set, &config.feature_groups[set]))
            .find_map(|(name, set)| {
                if set.at_least_one && set.members.len() == 1 && !set.members
                    .is_disjoint(&combination.always_on) {
                    Some(name)
                } else {
                    None
                }
            }) {
            Err(anyhow!(
                r#"Package "{}"'s configuration contains combinations whose always-on features intersect with the at-least-one set "{}", which contains only one element!"#,
                package.name,
                set,
            ))
        } else if let Some(feature) = combination.always_on.iter().find(|&&feature| !package.features.contains_key(feature)) {
            Err(anyhow!(
                r#"Package "{}"'s configuration's `always-on` section contains an undefined feature "{}"!"#,
                package.name,
                feature,
            ))
        } else {
            Ok(())
        }
    })
}
