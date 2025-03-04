use currency::platform::{Nls, Stable};
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
    fn balance(&self, oracle: Addr) -> Result<CoinStable>;

    /// Distributes a reward amount to an Lpp
    ///
    /// If `reward` == 0 no messages nor events are generated.
    fn distribute(self, reward: Coin<Nls>) -> Result<MessageResponse>;
}

pub fn new_stub(lpp: Addr, querier: QuerierWrapper<'_>, env: &Env) -> impl Lpp {
    Stub::new(lpp, querier, env)
}
