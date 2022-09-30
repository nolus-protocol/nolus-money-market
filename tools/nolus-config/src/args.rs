use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

impl Args {
    pub fn parse() -> Self {
        <Self as clap::Parser>::parse()
    }
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    #[clap(alias = "curr", alias = "gen-curr")]
    /// Generate currencies package according to the passed specification.
    ///
    /// Aliases: gen-curr
    GenerateCurrencies {
        #[clap(short, long, value_parser = dir_path_parser)]
        output_dir: PathBuf,
    },
    #[clap(alias = "script", alias = "gen-script")]
    /// Generate setup script based on the passed template.
    ///
    /// Aliases: gen-script
    SetupScript,
    // SetupScript {
    //     #[clap(short, long, value_parser=path_parser)]
    //     deploy_template: PathBuf,
    // },
}

fn path_parser(input: &str) -> Result<PathBuf, clap::Error> {
    PathBuf::from(input).canonicalize().map_err(|_| {
        clap::error::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            "Provided path can't be resolved and converted to canonical one!",
        )
    })
}

fn dir_path_parser(input: &str) -> Result<PathBuf, clap::Error> {
    let path = path_parser(input)?;

    if !path.is_dir() {
        return Err(clap::error::Error::raw(
            clap::error::ErrorKind::ValueValidation,
            "Provided path doesn't point to a directory!",
        ));
    }

    Ok(path)
}
