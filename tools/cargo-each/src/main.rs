use std::{
    collections::BTreeSet,
    env::current_dir,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use clap::Parser as _;

use crate::subcommands::{Arguments, Parser, SubcommandArguments};

use self::{pipe::Pipe as _, subcommands::Tags};

mod check;
mod combinations_iter;
mod config;
mod iter_or_else_iter;
mod pipe;
mod subcommands;

fn main() -> Result<()> {
    let arguments: Parser = Parser::parse();

    let Arguments {
        rust_path,
        manifest_path,
        mode,
        tags: groups,
        github_actions_logging,
        subcommand: subcommand_args,
    } = arguments.process();

    let metadata = get_metadata(rust_path.as_ref(), manifest_path)
        .context("Error occurred while fetching the workspaces' metadata!")?;

    let groups: BTreeSet<&str> = groups.iter().map(String::as_str).collect();

    let groups: Tags<'_> = (!groups.is_empty()).then_some(&groups);

    let current_dir = current_dir().context("Error occurred while resolving current directory!")?;

    match subcommand_args {
        SubcommandArguments::Run(arguments) => subcommands::run_subcommand(
            rust_path,
            &metadata,
            current_dir,
            mode,
            groups,
            github_actions_logging,
            arguments,
        ),
        SubcommandArguments::List(arguments) => subcommands::list_subcommand(
            &metadata,
            current_dir,
            mode,
            groups,
            github_actions_logging,
            arguments,
        ),
    }
}

fn get_metadata<RustPath, ManifestPath>(
    rust_path: Option<RustPath>,
    manifest_path: Option<ManifestPath>,
) -> Result<Metadata>
where
    RustPath: AsRef<OsStr>,
    ManifestPath: Into<PathBuf>,
{
    let mut command = MetadataCommand::new();

    if let Some(rust_path) = rust_path {
        let rust_path = rust_path.as_ref();

        command
            .cargo_path(build_cargo_bin_path(rust_path))
            .env("PATH", rust_path)
    } else {
        &mut command
    }
    .pipe_if_some(manifest_path, MetadataCommand::manifest_path)
    .exec()
    .context("Executing `cargo metadata` failed!")
}

fn build_cargo_bin_path<RustPath>(rust_path: &RustPath) -> PathBuf
where
    RustPath: AsRef<OsStr> + ?Sized,
{
    AsRef::<Path>::as_ref(rust_path.as_ref()).join(
        const {
            if cfg!(unix) {
                "cargo"
            } else if cfg!(windows) {
                "cargo.exe"
            } else {
                unimplemented!()
            }
        },
    )
}
