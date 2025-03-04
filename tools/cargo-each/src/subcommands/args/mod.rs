use std::{ffi::OsString, path::PathBuf};

use super::{Mode, list::Arguments as ListArguments, run::Arguments as RunArguments};

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
        long = "tag",
        help = "Select only packages containing all tags."
    )]
    tags: Vec<String>,
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
            tags,
            github_actions_logging,
            subcommand:
                CommandCallType::CargoCall(subcommand) | CommandCallType::DirectCall(subcommand),
        } = self;

        Arguments::new(
            cargo_path,
            manifest_path,
            workspace,
            package,
            tags,
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
            short = '1',
            long,
            help = "Indicates at most one combination will be selected."
        )]
        exact: bool,
        #[arg(short = 'x', long)]
        external_command: bool,
        #[arg(long, visible_alias = "debug")]
        print_command: bool,
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
                print_command,
                pass_package_manifest,
                pass_package_name,
                subcommand,
                args,
            } => SubcommandArguments::Run(RunArguments {
                exact,
                external_command,
                print_command,
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
    pub tags: Vec<String>,
    pub github_actions_logging: bool,
    pub subcommand: SubcommandArguments,
}

impl Arguments {
    fn new(
        cargo_path: PathBuf,
        manifest_path: Option<PathBuf>,
        workspace: bool,
        package: Option<String>,
        tags: Vec<String>,
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
            tags,
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
