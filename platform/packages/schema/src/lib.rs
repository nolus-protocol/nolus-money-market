use std::{env, fs::create_dir_all, io::Error as IoError, path::PathBuf};

use thiserror::Error as ThisError;

pub fn prep_out_dir() -> Result<PathBuf, Error> {
    let mut out_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or(Error::NoDirProvided)?;

    if out_dir.try_exists()? {
        out_dir.push("schema");

        create_dir_all(&out_dir)?;

        Ok(out_dir)
    } else {
        Err(Error::DirNotExist(out_dir.display().to_string()))
    }
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Directory not provided")]
    NoDirProvided,

    #[error("{0}")]
    Create(#[from] IoError),

    #[error(r#"The path "{0}" does not exist"#)]
    DirNotExist(String),
}
