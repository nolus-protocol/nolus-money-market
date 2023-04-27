use thiserror::Error;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Error)]
#[error("[Platform] Unreachable!")]
pub enum Never {}

pub fn safe_unwrap<T>(result: Result<T, Never>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

pub fn safe_unwrap_err<E>(result: Result<Never, E>) -> E {
    match result {
        Ok(never) => match never {},
        Err(error) => error,
    }
}
