#![cfg(not(target_arch = "wasm32"))]

use std::{
    borrow::Cow,
    env::{current_dir, var_os},
    fs::{create_dir, remove_dir_all, File},
    io::{stdin, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result as AnyResult};
use serde::Deserialize;
use serde_json::Deserializer;

use nolus_config::{CurrencyTemplate, GroupTemplate, ModuleName, Template};

use crate::{
    args::{Args, Subcommand},
    currencies::Currencies,
};

pub mod args;
pub mod currencies;

const LIB_POSTSCRIPT: &str = include_str!("../templates/lib_postscript.plain.txt");

const CURRENCY_POSTSCRIPT: &str = include_str!("../templates/currency_postscript.plain.txt");

const GROUP_POSTSCRIPT: &str = include_str!("../templates/group_postscript.plain.txt");

fn main() -> AnyResult<()> {
    let args: Args = Args::parse();

    match args.subcommand {
        Subcommand::GenerateCurrencies { output_dir } => generate_currencies(output_dir)?,
        Subcommand::SetupScript { .. } => unimplemented!(),
    }

    Ok(())
}

fn generate_currencies(output_dir: PathBuf) -> AnyResult<()> {
    let currencies: Currencies = read_currencies().context("Couldn't read currencies!")?;

    clean_output_dir(&output_dir).context("Couldn't clean up directory!")?;

    let currencies_dir = currencies_dir(output_dir.clone())?;

    let groups_dir = groups_dir(output_dir.clone())?;

    let mut file_paths = vec![];

    let mut lib_rs =
        create_lib_rs(output_dir, &mut file_paths).context("Couldn't create `lib.rs` file!")?;

    lib_rs.write_all(
        b"//! Auto-generated. Not intended for manual editing!\n\npub mod currencies {",
    )?;

    write_down_currencies(&currencies, currencies_dir, &mut lib_rs, &mut file_paths)?;

    lib_rs.write_all(b"\n}\n\npub  mod  groups {")?;

    write_down_groups(&currencies, groups_dir, &mut lib_rs, &mut file_paths)?;

    lib_rs.write_all(b"}\n")?;

    let lib_postscript: &[u8] = LIB_POSTSCRIPT.trim().as_bytes();

    if !lib_postscript.is_empty() {
        lib_rs.write_all(b"\n")?;

        lib_rs.write_all(lib_postscript)?;
    }

    std::process::Command::new({
        let mut path = var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("~/.cargo"));

        path.push("bin");
        path.push("rustfmt");

        path
    })
    .current_dir(current_dir().context("Couldn't fetch current working directory!")?)
    .args(["--emit", "files"])
    .args(file_paths)
    .spawn()
    .context("Couldn't spawn `rustfmt` process!")?
    .wait()
    .context("`rustfmt` process exited unexpectedly!")?;

    Ok(())
}

fn read_currencies() -> AnyResult<Currencies> {
    Currencies::deserialize(&mut Deserializer::from_reader(
        &mut {
            let mut buf = Vec::new();

            stdin().read_to_end(&mut buf)?;

            buf
        }
        .as_slice(),
    ))
    .map_err(Into::into)
}

fn create_lib_rs(mut output_dir: PathBuf, file_paths: &mut Vec<PathBuf>) -> AnyResult<File> {
    output_dir.push("lib.rs");

    let lib_rs = File::create(&output_dir)?;

    file_paths.push(output_dir);

    Ok(lib_rs)
}

fn clean_output_dir(output_dir: &Path) -> AnyResult<()> {
    if output_dir.exists() {
        assert!(
            output_dir.is_dir(),
            "Path for output directory points to existing file! Please remove file to proceed!"
        );

        remove_dir_all(output_dir)?;
    }

    create_dir(output_dir)?;

    Ok(())
}

fn currencies_dir(mut output_dir: PathBuf) -> AnyResult<PathBuf> {
    output_dir.push("currencies");

    create_dir(&output_dir)?;

    Ok(output_dir)
}

fn groups_dir(mut output_dir: PathBuf) -> AnyResult<PathBuf> {
    output_dir.push("groups");

    create_dir(&output_dir)?;

    Ok(output_dir)
}

fn write_down_currencies(
    currencies: &Currencies,
    currencies_dir: PathBuf,
    lib_rs: &mut File,
    file_paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    let postscript = CURRENCY_POSTSCRIPT.trim();

    currencies.currencies_iter().try_for_each(move |currency| {
        write_down_by_template::<CurrencyTemplate, { CurrencyTemplate::ARRAY_SIZE }>(
            currencies_dir.clone(),
            lib_rs,
            &currency,
            file_paths,
            postscript,
        )
    })
}

fn write_down_groups(
    currencies: &Currencies,
    groups_dir: PathBuf,
    lib_rs: &mut File,
    file_paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    let postscript = GROUP_POSTSCRIPT.trim();

    currencies.groups_iter().try_for_each(move |group| {
        write_down_by_template::<GroupTemplate, { GroupTemplate::ARRAY_SIZE }>(
            groups_dir.clone(),
            lib_rs,
            &group,
            file_paths,
            postscript,
        )
    })
}

fn write_down_by_template<T, const N: usize>(
    dir_path: PathBuf,
    lib_rs: &mut File,
    data: &T::SubstitutionData,
    file_paths: &mut Vec<PathBuf>,
    postscript: &str,
) -> AnyResult<()>
where
    T: Template<N>,
    T::SubstitutionData: ModuleName,
{
    let name = data.module_name();

    write_down_segments(
        dir_path,
        &name,
        &T::substitute(data),
        file_paths,
        postscript,
    )?;

    lib_rs.write_all(b"\n\tpub  mod  ")?;
    lib_rs.write_all(name.as_bytes())?;
    lib_rs.write_all(b";").map_err(Into::into)
}

fn write_down_segments(
    mut dir_path: PathBuf,
    module_name: &str,
    segments: &[Cow<str>],
    file_paths: &mut Vec<PathBuf>,
    postscript: &str,
) -> AnyResult<()> {
    dir_path.push(module_name);

    dir_path.set_extension("rs");

    let mut module_file: File = File::create(&dir_path)?;

    file_paths.push(dir_path);

    module_file.write_all(b"//! Auto-generated. Not intended for manual editing!\n\n")?;

    segments
        .iter()
        .try_for_each(|segment| module_file.write_all(segment.as_bytes()))?;

    module_file
        .write_all(postscript.as_bytes())
        .map_err(Into::into)
}
