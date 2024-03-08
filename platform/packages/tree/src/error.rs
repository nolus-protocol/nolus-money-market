#[derive(Debug, thiserror::Error)]
#[error("")]
pub enum Error {
    TreeTooBig,
}
