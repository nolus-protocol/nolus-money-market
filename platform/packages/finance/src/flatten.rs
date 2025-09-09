// TODO Switch to Result::flatten once migrate to Rust 1.89
pub trait Flatten<T, E> {
    fn flatten_pre_1_89(self) -> Result<T, E>;
}

impl<T, E> Flatten<T, E> for Result<Result<T, E>, E> {
    fn flatten_pre_1_89(self) -> Result<T, E> {
        match self {
            Ok(inner) => inner,
            Err(e) => Err(e),
        }
    }
}
