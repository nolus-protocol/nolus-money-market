use currency::NlsPlatform;
#[cfg(feature = "impl")]
use currency::{dex::Lpns, Currency};
use error::Result;
use finance::coin::Coin;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use stub::LppStub;

use crate::msg::LppBalanceResponse;

pub use crate::nlpn::NLpn;
pub use crate::usd::{CoinUsd, Usd, UsdGroup};

pub mod error;
pub mod msg;
mod nlpn;
mod stub;
#[cfg(feature = "testing")]
pub mod test;
mod usd;

pub trait Lpp
where
    Self: AsRef<Self>,
{
    fn balance(&self) -> Result<LppBalanceResponse>;

    /// Distributes a reward amount to an Lpp
    ///
    /// If `reward` == 0 no messages nor events are generated.
    fn ditribute_rewards(self, reward: Coin<NlsPlatform>) -> Result<MessageResponse>;
}

pub fn new_stub<'a, 'q>(
    lpp: &'a Addr,
    querier: &'q QuerierWrapper<'q>,
    env: &'a Env,
) -> impl Lpp + 'a
where
    'q: 'a,
{
    LppStub::new(lpp, querier, env)
}

#[cfg(feature = "impl")]
pub fn into_usd<Lpn>(amount: Coin<Lpn>) -> Coin<Usd>
where
    Lpn: Currency,
{
    use finance::coin::Amount;

    debug_assert_eq!(currency::validate_member::<Lpn, Lpns>(), Ok(()));

    Into::<Amount>::into(amount).into()
}
