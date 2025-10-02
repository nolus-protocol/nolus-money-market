use currency::{
    SymbolRef,
    platform::{Nls, Stable},
};
use error::Result;
use finance::coin::Coin;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use stub::Stub;

pub use crate::nlpn::NLpn;

pub mod error;
pub mod msg;
mod nlpn;
mod stub;
#[cfg(feature = "testing")]
pub mod test;

pub type CoinStable = Coin<Stable>;

pub trait Lpp {
    /// Return the total value in the stable currency
    fn balance<'stable_ticker>(
        &self,
        oracle: Addr,
        stable_ticker: SymbolRef<'stable_ticker>,
    ) -> Result<CoinStable>;

    /// Distributes a reward amount to an Lpp
    ///
    /// If `reward` == 0 no messages nor events are generated.
    fn distribute(self, reward: Coin<Nls>) -> Result<MessageResponse>;
}

pub fn new_stub<'querier, 'stable_ticker, 'env>(
    lpp: Addr,
    querier: QuerierWrapper<'querier>,
    env: &'env Env,
) -> impl Lpp + use<'querier, 'stable_ticker, 'env> {
    Stub::new(lpp, querier, env)
}
