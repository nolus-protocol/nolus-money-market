#[cfg(any(feature = "contract", test))]
pub(crate) use self::config::migrate;
#[cfg(any(feature = "contract", test))]
pub use self::{config::Config, deposit::Deposit, total::Total};

#[cfg(any(feature = "contract", test))]
mod config;
#[cfg(any(feature = "contract", test))]
mod deposit;
#[cfg(any(feature = "contract", test))]
mod total;
