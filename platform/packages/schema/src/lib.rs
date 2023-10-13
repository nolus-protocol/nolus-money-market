use std::{env, fs::create_dir_all, io::Error as IoError, path::PathBuf};

use thiserror::Error as ThisError;

pub fn prep_out_dir() -> Result<PathBuf, Error> {
    let mut args = env::args();
    args.next();
    let base_out_dir = args.next().ok_or(Error::NoDirProvided())?;

    let mut out_dir = PathBuf::new();
    out_dir.push(base_out_dir);
    if out_dir.try_exists()? {
        out_dir.push("schema");
        create_dir_all(&out_dir)?;
        Ok(out_dir)
    } else {
        use std::fmt::Write;
        let mut dir_str = String::new();
        write!(&mut dir_str, "{}", out_dir.display()).unwrap();
        Err(Error::DirNotExist(dir_str))
    }
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Directory not provided")]
    NoDirProvided(),

    #[error("{0}")]
    Create(#[from] IoError),

    #[error("The path '{0}' does not exist")]
    DirNotExist(String),
}
