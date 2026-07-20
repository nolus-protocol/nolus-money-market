use thiserror::Error;

use currency::Group;
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::SwapTarget;
use platform::{remote::Account as RemoteAccount, trx::Transaction};
use sdk::{api::ProtobufAny, cosmwasm_std::StdError};

#[cfg(feature = "impl")]
use finance::instant::Instant;

#[cfg(feature = "impl")]
use crate::SwapTask;

/// Yields the `Transport` a swap task should use.
///
/// Supplied by the host contract so a swap workflow stays generic over the DEX
/// it runs against.
#[cfg(feature = "impl")]
pub trait Factory {
    type TransportImpl<'this>: Transport
    where
        Self: 'this;

    /// The `Transport` to carry out `task`'s swap, as chosen for the moment `now`.
    fn transport<'task, Task>(&self, task: &'task Task, now: Instant) -> Self::TransportImpl<'task>
    where
        Task: SwapTask;
}

pub type SwapPathSlice<'a, G> = &'a [SwapTarget<G>];

/// A swap-exact-in against a specific DEX: the request messages it is made of
/// and the amount read back from its responses.
///
/// Implemented once per supported DEX.
pub trait Transport {
    /// `swap_path` should be a non-empty list
    ///
    /// `GIn` - the group of the input token
    /// `GOut` - the group of the output token
    /// `GSwap` - the group common for all tokens in the swap path
    fn build_request<GIn, GOut, GSwap>(
        trx: &mut Transaction,
        sender: RemoteAccount,
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
        I: Iterator<Item = ProtobufAny>;
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
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
