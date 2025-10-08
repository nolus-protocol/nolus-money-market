use thiserror::Error;

use currency::Group;
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::SwapTarget;
use platform::{ica::HostAccount, trx::Transaction};
use sdk::{cosmos_sdk_proto::Any as CosmosAny, cosmwasm_std::StdError};

pub type SwapPathSlice<'a, G> = &'a [SwapTarget<G>];

pub trait ExactAmountIn {
    /// `swap_path` should be a non-empty list
    ///
    /// `GIn` - the group of the input token
    /// `GOut` - the group of the output token
    /// `GSwap` - the group common for all tokens in the swap path
    fn build_request<GIn, GOut, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        amount_in: &CoinDTO<GIn>,
        min_amount_out: &CoinDTO<GOut>,
        swap_path: SwapPathSlice<'_, GSwap>,
    ) -> Result<()>
    where
        GIn: Group,
        GOut: Group,
        GSwap: Group;

    fn parse_response<I>(trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = CosmosAny>;
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Swap] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Swap] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Swap] {0}")]
    Std(String),

    #[error("[Swap] The value {0} is an invalid amount")]
    InvalidAmount(String),

    #[error("[Swap] Expected response to {0} is not found")]
    MissingResponse(String),
}

impl From<StdError> for Error {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}
