use thiserror::Error;

use currency::Group;
use platform::batch::Batch;
use remote_lease::swap::SwapParams;
use sdk::cosmwasm_std::StdError;

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
    type TopG: Group;
    type TransportImpl<'this>: Transport<Self::TopG>
    where
        Self: 'this;

    /// The `Transport` to carry out `task`'s swap, as chosen for the moment `now`.
    fn transport<'task, Task>(&self, task: &'task Task, now: Instant) -> Self::TransportImpl<'task>
    where
        Task: SwapTask;
}

/// A swap-exact-in against a specific DEX: the request messages it is made of
/// and the amount read back from its responses.
///
/// Implemented once per supported DEX. The swap parameters arrive already
/// widened to `TopG`, so the transport only forwards them to its DEX.
pub trait Transport<TopG>
where
    TopG: Group,
{
    fn swap(self, params: SwapParams<TopG, TopG>) -> Result<Batch>;
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
