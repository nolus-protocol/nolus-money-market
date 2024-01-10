use thiserror::Error;

use currency::Group;
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::SwapPath;
use platform::{ica::HostAccount, trx::Transaction};
use sdk::{cosmos_sdk_proto::Any, cosmwasm_std::StdError};

pub trait ExactAmountIn {
    /// `swap_path` should be a non-empty list
    ///
    /// `GIn` - the group of the input token
    /// `GSwap` - the group common for all tokens in the swap path
    fn build<GIn, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        swap_path: &SwapPath,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group;

    fn parse<I>(trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = Any>;

    #[cfg(feature = "testing")]
    fn build_resp(amount_out: Amount) -> Any;
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Swap] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Swap] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Swap] {0}")]
    Std(#[from] StdError),

    #[error("[Swap] The value {0} is an invalid amount")]
    InvalidAmount(String),

    #[error("[Swap] Expected response to {0} is not found")]
    MissingResponse(String),
}