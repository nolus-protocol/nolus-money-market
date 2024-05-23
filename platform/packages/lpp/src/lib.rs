use currency::NlsPlatform;
use error::Result;
use finance::coin::Coin;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use stub::Stub;

pub use crate::{
    nlpn::NLpn,
    stable::{CoinStable, Stable, StableCurrencyGroup},
};

pub mod error;
pub mod msg;
mod nlpn;
mod stable;
mod stub;
#[cfg(feature = "testing")]
pub mod test;

pub trait Lpp {
    /// Return the total value in the stable currency
    fn balance(&self, oracle: Addr) -> Result<CoinStable>;

    /// Distributes a reward amount to an Lpp
    ///
    /// If `reward` == 0 no messages nor events are generated.
    fn ditribute(self, reward: Coin<NlsPlatform>) -> Result<MessageResponse>;
}

pub fn new_stub<'a, 'q>(lpp: Addr, querier: QuerierWrapper<'q>, env: &'a Env) -> impl Lpp + 'a
where
    'q: 'a,
{
    Stub::new(lpp, querier, env)
}
