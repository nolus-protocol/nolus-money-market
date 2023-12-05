use std::{
    collections::BTreeSet,
    io::{stdout, Write},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

const CARGO_SUBCOMMAND_NAME: &str = {
    const PACKAGE_NAME: &[u8] = env!("CARGO_PKG_NAME").as_bytes();

    const CARGO_SUBCOMMAND_PREFIX: &[u8] = b"cargo-";

    const SUBCOMMAND_NAME_LENGTH: usize = PACKAGE_NAME.len() - CARGO_SUBCOMMAND_PREFIX.len();

    const SUBCOMMAND_NAME_ARRAY: [u8; SUBCOMMAND_NAME_LENGTH] = {
        let mut array = [0; SUBCOMMAND_NAME_LENGTH];

        let mut index = 0;

        while index < SUBCOMMAND_NAME_LENGTH {
            array[index] = PACKAGE_NAME[CARGO_SUBCOMMAND_PREFIX.len() + index];

            index += 1;
        }

        array
    };

    if PACKAGE_NAME.len() <= CARGO_SUBCOMMAND_PREFIX.len() {
        unimplemented!()
    }

    {
        let mut index = 0;

        while index < CARGO_SUBCOMMAND_PREFIX.len() {
            if PACKAGE_NAME[index] != CARGO_SUBCOMMAND_PREFIX[index] {
                unimplemented!()
            }

            index += 1;
        }
    }

    if let Ok(s) = std::str::from_utf8(&SUBCOMMAND_NAME_ARRAY) {
        s
    } else {
        unreachable!()
    }
};

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: FeaturesCommand,
}

#[derive(Debug, Subcommand)]
enum FeaturesCommand {
    #[command(subcommand, name = CARGO_SUBCOMMAND_NAME)]
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
