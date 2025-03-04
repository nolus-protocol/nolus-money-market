use std::path::PathBuf;

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, Package};

use crate::{combinations_iter, config::deserialize_config_if_any};

use super::{Mode, Tags, get_packages_iter};

pub(crate) struct Arguments {}

pub(crate) fn subcommand(
    metadata: &Metadata,
    current_dir: PathBuf,
    mode: Mode,
    groups: Tags<'_>,
    github_actions_logging: bool,
    Arguments {}: Arguments,
) -> Result<()> {
    get_packages_iter(metadata, current_dir, mode).and_then(|mut packages| {
        packages.try_for_each(|package| list_for_package(groups, github_actions_logging, package))
    })
}

fn list_for_package(
    groups: Tags<'_>,
    github_actions_logging: bool,
    package: &Package,
) -> Result<()> {
    let maybe_config = deserialize_config_if_any(package)?;

    let features_combinations =
        combinations_iter::package_combinations(package, maybe_config.as_ref(), groups)
            .context("Error occurred while constructing combinations!")?;

    println!(
        r#"{}Combinations of package "{}":"#,
        if github_actions_logging {
            "##[group]"
        } else {
            ""
        },
        package.name
    );

    features_combinations.for_each(|combination| println!("\t`{combination}`"));

    if github_actions_logging {
        println!("##[endgroup]");
    }

    Ok(())
}
