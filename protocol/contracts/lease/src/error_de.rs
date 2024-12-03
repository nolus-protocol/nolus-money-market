use std::any::type_name;

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ErrorDe {
    #[error("[Lease] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),
}

impl ErrorDe {
    pub fn broken_invariant_if<T>(check: bool, msg: &str) -> Result<(), ErrorDe> {
        if check {
            Err(Self::BrokenInvariant(type_name::<T>().into(), msg.into()))
        } else {
            Ok(())
        }
    }
}
