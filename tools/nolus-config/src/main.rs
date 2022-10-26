use std::{
    ffi::OsStr,
    fs::{create_dir, remove_dir_all, remove_file, File},
    io::{stdin, Read, Write},
    path::{Path, PathBuf},
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

    clean_output_dir(&output_dir)?;

    let currency_dir = currency_dir(output_dir.clone())?;

    let group_dir = group_dir(output_dir.clone())?;

    let mut lib_rs = create_lib_rs(output_dir)?;

    let generated = currencies.generate("crate::currencies");

    lib_rs.write_all(b"pub mod currencies {")?;

    for currency_source in generated.currencies.iter() {
        write_currency(currency_dir.clone(), &mut lib_rs, currency_source)?;
    }

    lib_rs.write_all(b"}\n\npub mod groups {")?;

    for group_source in generated.groups.iter() {
        write_group(group_dir.clone(), &mut lib_rs, group_source)?;
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

fn clean_output_dir(output_dir: &Path) -> Result<(), Error> {
    for entry in output_dir.read_dir()? {
        let entry = entry?;

        if ![OsStr::new("."), OsStr::new("..")].contains(&entry.file_name().as_os_str()) {
            (if entry.file_type()?.is_dir() {
                remove_dir_all
            } else {
                remove_file
            }(entry.path()))?;
        }
    }

    Ok(())
}

fn currency_dir(mut output_dir: PathBuf) -> Result<PathBuf, Error> {
    output_dir.push("currencies");

    create_dir(&output_dir)?;

    Ok(output_dir)
}

fn group_dir(mut output_dir: PathBuf) -> Result<PathBuf, Error> {
    output_dir.push("groups");

    create_dir(&output_dir)?;

    Ok(output_dir)
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
    mut currency_path: PathBuf,
    lib_rs: &mut File,
    currency_source: CurrencyFilenameSource,
) -> Result<(), Error> {
    lib_rs.write_all(format!("\n\tpub mod {};\n", currency_source.filename()).as_bytes())?;

    currency_path.push(currency_source.filename());

    currency_path.set_extension("rs");

    currency_source
        .generate_source(File::create(currency_path)?)
        .map_err(Into::into)
}

fn write_group(
    mut group_path: PathBuf,
    lib_rs: &mut File,
    group_source: GroupFilenameSource,
) -> Result<(), Error> {
    lib_rs.write_all(format!("\n\tpub mod {};\n", group_source.filename()).as_bytes())?;

    group_path.push(group_source.filename());

    group_path.set_extension("rs");

    group_source
        .generate_source(File::create(group_path)?)
        .map_err(Into::into)
}
