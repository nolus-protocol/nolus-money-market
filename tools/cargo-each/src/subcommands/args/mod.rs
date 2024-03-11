use std::{ffi::OsString, path::PathBuf};

use super::{list::Arguments as ListArguments, run::Arguments as RunArguments, Mode};

mod consts;

#[derive(Debug, clap::Parser)]
pub(crate) struct Parser {
    #[arg(global = true, long, env = "CARGO", default_value_os_t = PathBuf::from("cargo"))]
    cargo_path: PathBuf,
    #[arg(global = true, long)]
    manifest_path: Option<PathBuf>,
    #[arg(global = true, short = 'a', long)]
    workspace: bool,
    #[arg(global = true, short, long, conflicts_with = "workspace")]
    package: Option<String>,
    #[arg(
        global = true,
        short,
        long = "group",
        help = "Select only packages containing all groups."
    )]
    groups: Vec<String>,
    #[arg(global = true, long, visible_alias = "gha-log")]
    github_actions_logging: bool,
    #[command(subcommand)]
    subcommand: CommandCallType,
}

impl Parser {
    pub fn process(self) -> Arguments {
        let Self {
            cargo_path,
            manifest_path,
            workspace,
            package,
            groups,
            github_actions_logging,
            subcommand:
                CommandCallType::CargoCall(subcommand) | CommandCallType::DirectCall(subcommand),
        } = self;

        Arguments::new(
            cargo_path,
            manifest_path,
            workspace,
            package,
            groups,
            github_actions_logging,
            subcommand.process(),
        )
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    List {},
    Run {
        #[arg(
            short,
            long,
            help = "Indicates at most one combination will be selected."
        )]
        exact: bool,
        #[arg(short, long)]
        external_command: bool,
        #[arg(long)]
        pass_package_manifest: bool,
        #[arg(long)]
        pass_package_name: bool,
        subcommand: OsString,
        args: Vec<OsString>,
    },
}

impl Subcommand {
    pub(crate) fn process(self) -> SubcommandArguments {
        match self {
            Subcommand::List {} => SubcommandArguments::List(ListArguments {}),
            Subcommand::Run {
                exact,
                external_command,
                pass_package_manifest,
                pass_package_name,
                subcommand,
                args,
            } => SubcommandArguments::Run(RunArguments {
                exact,
                external_command,
                pass_package_manifest,
                pass_package_name,
                subcommand,
                args,
            }),
        }
    }
}

pub(crate) struct Arguments {
    pub cargo_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub mode: Mode,
    pub groups: Vec<String>,
    pub github_actions_logging: bool,
    pub subcommand: SubcommandArguments,
}

impl Arguments {
    fn new(
        cargo_path: PathBuf,
        manifest_path: Option<PathBuf>,
        workspace: bool,
        package: Option<String>,
        groups: Vec<String>,
        github_actions_logging: bool,
        subcommand: SubcommandArguments,
    ) -> Self {
        Self {
            cargo_path,
            manifest_path,
            mode: workspace
                .then_some(Mode::Workspace)
                .or_else(|| package.map(Mode::Package))
                .unwrap_or(Mode::Subdirectories),
            groups,
            github_actions_logging,
            subcommand,
        }
    }
}

pub(crate) enum SubcommandArguments {
    Run(RunArguments),
    List(ListArguments),
}

#[derive(Debug, clap::Subcommand)]
enum CommandCallType {
    #[command(subcommand, name = consts::CARGO_SUBCOMMAND_NAME)]
    CargoCall(Subcommand),
    #[command(flatten)]
    DirectCall(Subcommand),
}
