#[cfg(feature = "impl")]
use currency::{dex::Lpns, Currency};
use error::Result;
#[cfg(feature = "impl")]
use finance::coin::Coin;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use stub::LppStub;

use crate::msg::LppBalanceResponse;

pub use crate::nlpn::NLpn;
pub use crate::usd::{Usd, UsdGroup};

pub mod error;
pub mod msg;
mod nlpn;
mod stub;
mod usd;

pub trait Lpp {
    fn balance(&self) -> Result<LppBalanceResponse>;
}

pub fn new_stub<'a>(lpp: Addr, querier: &'a QuerierWrapper<'a>) -> impl Lpp + 'a {
    LppStub::new(lpp, querier)
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
