/// Never instantiated error
/// TODO Replace with `!` once stabilized
///
/// Credit: cosmwasm_std::Never
#[cfg_attr(test, derive(Debug))]
pub enum Never {}
pub fn safe_unwrap<T>(res: Result<T, Never>) -> T {
    match res {
        Ok(value) => value,
        Err(err) => match err {},
    }
}
