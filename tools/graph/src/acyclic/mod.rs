use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

pub mod double_edged;

#[derive(Debug, PartialEq, Eq)]
pub struct CycleCreation;

impl Display for CycleCreation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Cycle would have been created when adding edge!")
    }
}

impl Error for CycleCreation {}
