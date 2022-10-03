use std::{
    fs::{create_dir, remove_dir_all, File},
    io::{stdin, Read, Write},
    path::PathBuf,
};

use serde::Deserialize;
use serde_json::Deserializer;

use crate::{
    args::{Args, Subcommand},
    currencies::{Currencies, CurrencyFilenameSource, GroupFilenameSource},
    error::Error,
};

const CURRENCY_TEMPLATE: &str = include_str!("currency.template.txt");

const GROUP_TEMPLATE: &str = include_str!("groups.template.txt");

pub mod args;
pub mod currencies;
pub mod error;

fn main() -> Result<(), Error> {
    let args: Args = Args::parse();

    match args.subcommand {
        Subcommand::GenerateCurrencies { output_dir } => generate_currencies(output_dir)?,
        Subcommand::SetupScript { .. } => {}
    }

    Ok(())
}

fn generate_currencies(output_dir: PathBuf) -> Result<(), Error> {
    let currencies: Currencies = read_currencies()?;

    let currency_dir = currency_dir(&output_dir)?;

    let group_dir = group_dir(&output_dir)?;

    let mut lib_rs = create_lib_rs(output_dir)?;

    let generated = currencies.generate("crate::currencies");

    lib_rs.write_all(b"pub mod currencies {")?;

    for currency_source in generated.currencies.iter() {
        write_currency(&currency_dir, &mut lib_rs, currency_source)?;
    }

    lib_rs.write_all(b"}\n\npub mod groups {")?;

    for group_source in generated.groups.iter() {
        write_group(&group_dir, &mut lib_rs, group_source)?;
    }

    lib_rs.write_all(b"}\n").map_err(Into::into)
}

fn read_currencies() -> Result<Currencies, Error> {
    <Currencies as Deserialize>::deserialize(&mut Deserializer::from_reader(
        &mut {
            let mut buf = Vec::new();

            stdin().read_to_end(&mut buf)?;

            buf
        }
        .as_slice(),
    ))
    .map_err(Error::from_deserialization)
}

fn currency_dir(output_dir: &PathBuf) -> Result<PathBuf, Error> {
    let mut currency_dir = output_dir.clone();

    currency_dir.push("currencies");

    if currency_dir.exists() {
        remove_dir_all(&currency_dir)?;
    }

    create_dir(&currency_dir)?;

    Ok(currency_dir)
}

fn group_dir(output_dir: &PathBuf) -> Result<PathBuf, Error> {
    let mut group_dir = output_dir.clone();

    group_dir.push("groups");

    if group_dir.exists() {
        remove_dir_all(&group_dir)?;
    }

    create_dir(&group_dir)?;

    Ok(group_dir)
}

fn create_lib_rs(output_dir: PathBuf) -> Result<File, Error> {
    File::create({
        let mut lib_rs_path = output_dir;

        lib_rs_path.push("lib");

        lib_rs_path.set_extension("rs");

        lib_rs_path
    })
    .map_err(Into::into)
}

fn write_currency(
    currency_dir: &PathBuf,
    lib_rs: &mut File,
    currency_source: CurrencyFilenameSource,
) {
    lib_rs.write_all(format!("\n\tpub mod {};\n", currency_source.filename()).as_bytes())?;

    let mut currency_path = currency_dir.clone();

    currency_path.push(currency_source.filename());

    currency_path.set_extension("rs");

    currency_source.generate_source(File::create(currency_path)?)?;
}

fn write_group(group_dir: &PathBuf, lib_rs: &mut File, group_source: GroupFilenameSource) {
    lib_rs.write_all(format!("\n\tpub mod {};\n", group_source.filename()).as_bytes())?;

    let mut group_path = group_dir.clone();

    group_path.push(group_source.filename());

    group_path.set_extension("rs");

    group_source.generate_source(File::create(group_path)?)?;
}
