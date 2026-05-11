use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(
        "the three remote-lease currencies (downpayment, lpn, asset) must be pairwise distinct"
    )]
    DuplicateLeaseCurrencies,

    #[error("swap input and output currencies must differ")]
    SameSwapCurrency,
}
