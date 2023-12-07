use std::{
    collections::BTreeSet,
    io::{stdout, Write},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod consts;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: FeaturesCommand,
}

#[derive(Debug, Subcommand)]
enum FeaturesCommand {
    #[command(subcommand, name = consts::CARGO_SUBCOMMAND_NAME)]
    CargoCall(Command),
    #[command(flatten)]
    DirectCall(Command),
}

#[derive(Debug, Subcommand)]
enum Command {
    List,
    Intersection {
        #[arg(value_parser = parse_features)]
        features: BTreeSet<String>,
    },
}

fn parse_features(s: &str) -> Result<BTreeSet<String>> {
    Ok(s.split(',')
        .filter(|&s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn main() -> Result<()> {
    let Args {
        command: FeaturesCommand::CargoCall(command) | FeaturesCommand::DirectCall(command),
    } = Args::parse();

    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to execute `cargo metadata`!")?;

    let package =  metadata
        .root_package()
        .context("Running on a outside an individual package! Running in a virtual workspace is not allowed!")?;

    match command {
        Command::List => concat_and_write(package.features.keys()),
        Command::Intersection { ref features } => concat_and_write(
            package
                .features
                .keys()
                .cloned()
                .collect::<BTreeSet<String>>()
                .intersection(features),
        ),
    }
}

fn concat_and_write<'r, I>(iter: I) -> Result<()>
where
    I: Iterator<Item = &'r String>,
{
    stdout()
        .write_all(iter.cloned().collect::<Vec<String>>().join(",").as_bytes())
        .context("Couldn't write result to STDOUT!")
}
